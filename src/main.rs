//! # CHIP-8 Emulator
//!
//! Reference: [Cowgod's Chip-8 Technical Reference](http://devernay.free.fr/hacks/chip8/C8TECH10.HTM)

use bytemuck::{Pod, Zeroable};
use cgmath::{prelude::*, Vector2, Vector3};
use clap::Parser;
use log::{error, info, warn};
use rodio::{source::SineWave, Source};
use rusty_chip8::{
    camera::{Camera, CameraUniform},
    error::{AppError, AppResult},
    renderer::Renderer,
    screen::Screen,
    world::World,
};
use std::{
    borrow::Cow,
    cell::RefCell,
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant},
};
use wgpu::util::DeviceExt;
use winit::{
    dpi::{LogicalSize, Size},
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    rom_path: String,
}

fn main() -> Result<(), AppError> {
    #[cfg(not(target_arch = "wasm32"))]
    let rom = {
        println!("Hello, CHIP-8!");

        let args = Args::parse();

        // Load ROM
        let file = File::open(&args.rom_path)?;
        let rom = BufReader::new(file);
        rom.bytes().map(|b| b.unwrap()).collect::<Vec<u8>>()
    };

    // let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    // let beep = SineWave::new(560.0f32)
    //     .take_duration(Duration::from_millis(200))
    //     .fade_in(Duration::from_millis(100));
    // let beep1 = stream_handle.play_raw(beep).unwrap();
    // beep1.set_volume(1.0);

    let event_loop = EventLoop::new().unwrap();

    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_inner_size(LogicalSize::new(640 * 2, 320 * 2));

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowBuilderExtWebSys;
        let canvas = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        builder = builder.with_canvas(Some(canvas));
    }
    let window = builder.build(&event_loop).unwrap();

    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        pollster::block_on(run(event_loop, window, rom));
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        wasm_bindgen_futures::spawn_local(run(event_loop, window));
    }

    Ok(())
}

async fn run(event_loop: EventLoop<()>, window: Window, rom: Vec<u8>) -> AppResult<()> {
    let mut surface_size = window.inner_size();
    surface_size.width = surface_size.width.max(1);
    surface_size.height = surface_size.height.max(1);

    let world = Rc::new(RefCell::new(World::new(surface_size)));
    let mut renderer = Renderer::create(&window, Rc::clone(&world), surface_size).await;

    let mut surface_configured = false;
    let window = &window;

    // Render timings
    const FRAME_TIME: i64 = 16_666;
    let start_time = Instant::now();
    let mut previous_time = 0i64;
    let mut elapsed_time = 0i64;
    let mut lag = 0i64;
    // let mut last_fps_update = 0i64;
    // let mut fps = 0u64;

    // Control
    let mut pressed_keys: [bool; 16] = [false; 16];
    let mut waiting_for_key: Option<usize> = None;
    let mut paused = false;
    let mut speed = 15;

    // Chip
    const INSTRUCTION_LEN: u16 = 2;
    let mut memory: [u8; 4096] = [0; 4096];
    let mut registers: [u8; 16] = [0; 16];
    let mut register_i: u16 = 0;

    let mut pc: u16 = 0x200;
    let mut stack: [u16; 16] = [0; 16];
    let mut sp: u8 = 0;
    let mut delay_timer: u8 = 0;
    let mut sound_timer: u8 = 0;

    const SPRITES: [[u8; 5]; 16] = [
        [0xF0, 0x90, 0x90, 0x90, 0xF0], // 0
        [0x20, 0x60, 0x20, 0x20, 0x70], // 1
        [0xF0, 0x10, 0xF0, 0x80, 0xF0], // 2
        [0xF0, 0x10, 0xF0, 0x10, 0xF0], // 3
        [0x90, 0x90, 0xF0, 0x10, 0x10], // 4
        [0xF0, 0x80, 0xF0, 0x10, 0xF0], // 5
        [0xF0, 0x80, 0xF0, 0x90, 0xF0], // 6
        [0xF0, 0x10, 0x20, 0x40, 0x40], // 7
        [0xF0, 0x90, 0xF0, 0x90, 0xF0], // 8
        [0xF0, 0x90, 0xF0, 0x10, 0xF0], // 9
        [0xF0, 0x90, 0xF0, 0x90, 0x90], // A
        [0xE0, 0x90, 0xE0, 0x90, 0xE0], // B
        [0xF0, 0x80, 0x80, 0x80, 0xF0], // C
        [0xE0, 0x90, 0x90, 0x90, 0xE0], // D
        [0xF0, 0x80, 0xF0, 0x80, 0xF0], // E
        [0xF0, 0x80, 0xF0, 0x80, 0x80], // F
    ];
    // Sprite data should be stored in the interpreter area of Chip-8 memory (0x000 to 0x1FF).
    for (i, sprite) in SPRITES.iter().enumerate() {
        for (j, &value) in sprite.iter().enumerate() {
            memory[i * 5 + j] = value;
        }
    }
    for (i, value) in rom.iter().enumerate() {
        memory[0x200 + i] = *value;
    }

    event_loop.run(move |event, target| {
        // Have the closure take ownership of the resources.
        // `event_loop.run` never returns, therefore we must do this to ensure
        // the resources are properly cleaned up.
        // let _ = (&instance, &adapter, &shader, &pipeline_layout);
        let _ = (&renderer);

        if let Event::WindowEvent {
            window_id: _,
            event,
        } = event
        {
            match event {
                WindowEvent::RedrawRequested => {
                    window.request_redraw();

                    if !surface_configured {
                        return;
                    }

                    let current_time = Instant::now().duration_since(start_time).as_micros() as i64;
                    elapsed_time = current_time - previous_time;

                    previous_time = current_time;

                    if !paused {
                        lag += elapsed_time;
                        while lag >= FRAME_TIME {
                            renderer.update();

                            if delay_timer > 0 {
                                delay_timer -= 1;
                            }
                            if sound_timer > 0 {
                                sound_timer -= 1;
                            }

                            lag -= FRAME_TIME;
                        }
                    }

                    match renderer.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            renderer.resize(renderer.surface_size());
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            error!("OutOfMemory");
                            target.exit();
                        }
                        Err(wgpu::SurfaceError::Timeout) => {
                            warn!("Surface timeout")
                        }
                    }

                    // fps += 1;
                    // if (current_time - last_fps_update) >= 1_000_000 {
                    //     println!("FPS: {}", fps);
                    //     fps = 0;
                    //     last_fps_update = current_time;
                    // }

                    // renderer.update();

                    for i in 0..speed {
                        if paused {
                            break;
                        }

                        // Execute instruction
                        let opcode =
                            (memory[pc as usize] as u16) << 8 | memory[pc as usize + 1] as u16;

                        // Variables
                        let nnn = opcode & 0x0FFF;
                        let nibble = opcode & 0x000F;
                        let x = ((opcode & 0x0F00) >> 8) as usize;
                        let y = ((opcode & 0x00F0) >> 4) as usize;
                        let kk = (opcode & 0x00FF) as u8;

                        // Decode opcode
                        match opcode & 0xF000 {
                            0x0000 => match opcode {
                                0x00E0 => {
                                    // 00E0 - CLS
                                    // Clear the display.
                                    world.borrow_mut().screen.clear();
                                }
                                0x00EE => {
                                    // 00EE - RET
                                    // Return from a subroutine.
                                    // The interpreter sets the program counter to the address at the top of the stack, then subtracts 1 from the stack pointer.
                                    sp -= 1;
                                    pc = stack[sp as usize];
                                }
                                _ => {
                                    // 0nnn - SYS addr
                                    // Jump to a machine code routine at nnn.
                                    // This instruction is only used on the old computers on which Chip-8 was originally implemented.
                                    // It is ignored by modern interpreters.
                                }
                            },
                            0x1000 => {
                                // 1nnn - JP addr
                                // Jump to location nnn.
                                // The interpreter sets the program counter to nnn.
                                pc = nnn;
                                continue;
                            }
                            0x2000 => {
                                // 2nnn - CALL addr
                                // Call subroutine at nnn.
                                // The interpreter increments the stack pointer, then puts the current PC on the top of the stack. The PC is then set to nnn.
                                stack[sp as usize] = pc;
                                sp += 1;
                                pc = nnn;
                                continue;
                            }
                            0x3000 => {
                                // 3xkk - SE Vx, byte
                                // Skip next instruction if Vx = kk.
                                // The interpreter compares register Vx to kk, and if they are equal, increments the program counter by 2.
                                if registers[x] == kk {
                                    pc += INSTRUCTION_LEN;
                                }
                            }
                            0x4000 => {
                                // 4xkk - SNE Vx, byte
                                // Skip next instruction if Vx != kk.
                                // The interpreter compares register Vx to kk, and if they are not equal, increments the program counter by 2.
                                if registers[x] != kk {
                                    pc += INSTRUCTION_LEN;
                                }
                            }
                            0x5000 => {
                                // 5xy0 - SE Vx, Vy
                                // Skip next instruction if Vx = Vy.
                                // The interpreter compares register Vx to register Vy, and if they are equal, increments the program counter by 2.
                                if registers[x] == registers[y] {
                                    pc += INSTRUCTION_LEN;
                                }
                            }
                            0x6000 => {
                                // 6xkk - LD Vx, byte
                                // Set Vx = kk.
                                // The interpreter puts the value kk into register Vx.
                                registers[x] = kk as u8;
                            }
                            0x7000 => {
                                // 7xkk - ADD Vx, byte
                                // Set Vx = Vx + kk.
                                // Adds the value kk to the value of register Vx, then stores the result in Vx.
                                registers[x] = registers[x].wrapping_add(kk as u8);
                            }
                            0x8000 => match nibble {
                                0x0000 => {
                                    // 8xy0 - LD Vx, Vy
                                    // Set Vx = Vy.
                                    // Stores the value of register Vy in register Vx.
                                    registers[x] = registers[y];
                                }
                                0x0001 => {
                                    // 8xy1 - OR Vx, Vy
                                    // Set Vx = Vx OR Vy.
                                    // Performs a bitwise OR on the values of Vx and Vy, then stores the result in Vx.
                                    registers[x] |= registers[y];
                                }
                                0x0002 => {
                                    // 8xy2 - AND Vx, Vy
                                    // Set Vx = Vx AND Vy.
                                    // Performs a bitwise AND on the values of Vx and Vy, then stores the result in Vx.
                                    registers[x] &= registers[y];
                                }
                                0x0003 => {
                                    // 8xy3 - XOR Vx, Vy
                                    // Set Vx = Vx XOR Vy.
                                    // Performs a bitwise exclusive OR on the values of Vx and Vy, then stores the result in Vx.
                                    registers[x] ^= registers[y];
                                }
                                0x0004 => {
                                    // 8xy4 - ADD Vx, Vy
                                    // Set Vx = Vx + Vy, set VF = carry.
                                    // The values of Vx and Vy are added together. If the result is greater than 8 bits (i.e., > 255,) VF is set to 1, otherwise 0.
                                    // Only the lowest 8 bits of the result are kept, and stored in Vx.
                                    let (result, overflow) =
                                        registers[x].overflowing_add(registers[y]);
                                    registers[x] = result;
                                    registers[0xF] = overflow as u8;
                                }
                                0x0005 => {
                                    // 8xy5 - SUB Vx, Vy
                                    // Set Vx = Vx - Vy, set VF = NOT borrow.
                                    // If Vx > Vy, then VF is set to 1, otherwise 0. Then Vy is subtracted from Vx, and the results stored in Vx.
                                    let (result, overflow) =
                                        registers[x].overflowing_sub(registers[y]);
                                    registers[x] = result;
                                    registers[0xF] = !overflow as u8;
                                }
                                0x0006 => {
                                    // 8xy6 - SHR Vx {, Vy}
                                    // Set Vx = Vx SHR 1.
                                    // If the least-significant bit of Vx is 1, then VF is set to 1, otherwise 0. Then Vx is divided by 2.
                                    registers[0xF] = registers[x] & 0x1;
                                    registers[x] >>= 1;
                                }
                                0x0007 => {
                                    // 8xy7 - SUBN Vx, Vy
                                    // Set Vx = Vy - Vx, set VF = NOT borrow.
                                    // If Vy > Vx, then VF is set to 1, otherwise 0. Then Vx is subtracted from Vy, and the results stored in Vx.
                                    let (result, overflow) =
                                        registers[y].overflowing_sub(registers[x]);
                                    registers[x] = result;
                                    registers[0xF] = !overflow as u8;
                                }
                                0x000E => {
                                    // 8xyE - SHL Vx {, Vy}
                                    // Set Vx = Vx SHL 1.
                                    // If the most-significant bit of Vx is 1, then VF is set to 1, otherwise to 0. Then Vx is multiplied by 2.
                                    registers[0xF] = (registers[x] & 0x80) >> 7;
                                    registers[x] <<= 1;
                                }
                                _ => unreachable!("Unknown opcode: {:#06X}", opcode),
                            },
                            0x9000 => {
                                // 9xy0 - SNE Vx, Vy
                                // Skip next instruction if Vx != Vy.
                                // The values of Vx and Vy are compared, and if they are not equal, the program counter is increased by 2.
                                if registers[x] != registers[y] {
                                    pc += INSTRUCTION_LEN;
                                }
                            }
                            0xA000 => {
                                // Annn - LD I, addr
                                // Set I = nnn.
                                // The value of register I is set to nnn.
                                register_i = nnn;
                            }
                            0xB000 => {
                                // Bnnn - JP V0, addr
                                // Jump to location nnn + V0.
                                // The program counter is set to nnn plus the value of V0.
                                pc = nnn + registers[0] as u16;
                                continue;
                            }
                            0xC000 => {
                                // Cxkk - RND Vx, byte
                                // Set Vx = random byte AND kk.
                                // The interpreter generates a random number from 0 to 255, which is then ANDed with the value kk.
                                // The results are stored in Vx.
                                registers[x] = rand::random::<u8>() & kk;
                            }
                            0xD000 => {
                                // Dxyn - DRW Vx, Vy, nibble
                                // Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision.
                                // The interpreter reads n bytes from memory, starting at the address stored in I.
                                // These bytes are then displayed as sprites on screen at coordinates (Vx, Vy).
                                // Sprites are XORed onto the existing screen.
                                // If this causes any pixels to be erased, VF is set to 1, otherwise it is set to 0.
                                // If the sprite is positioned so part of it is outside the coordinates of the display, it wraps around to the opposite side of the screen.

                                let width = 8u8; // 8 pixels
                                let height = nibble as u8;

                                registers[0xF] = 0;
                                for y_pixel in 0..height {
                                    let mut pixel = memory[register_i as usize + y_pixel as usize];
                                    for x_pixel in 0..width {
                                        if (pixel & 0x80) > 0 {
                                            if world.borrow_mut().screen.toggle(
                                                registers[x].wrapping_add(x_pixel),
                                                registers[y].wrapping_add(y_pixel),
                                            ) {
                                                registers[0xF] = 1;
                                            }
                                        }
                                        pixel <<= 1;
                                    }
                                }
                            }
                            0xE000 => match kk {
                                0x9E => {
                                    // Ex9E - SKP Vx
                                    // Skip next instruction if key with the value of Vx is pressed.
                                    // Checks the keyboard, and if the key corresponding to the value of Vx is currently in the down position, PC is increased by 2.
                                    if pressed_keys[registers[x] as usize] {
                                        pc += INSTRUCTION_LEN;
                                    }
                                }
                                0xA1 => {
                                    // ExA1 - SKNP Vx
                                    // Skip next instruction if key with the value of Vx is not pressed.
                                    // Checks the keyboard, and if the key corresponding to the value of Vx is currently in the up position, PC is increased by 2.
                                    if !pressed_keys[registers[x] as usize] {
                                        pc += INSTRUCTION_LEN;
                                    }
                                }
                                _ => unreachable!("Unknown opcode: {:#06X}", opcode),
                            },
                            0xF000 => match kk {
                                0x07 => {
                                    // Fx07 - LD Vx, DT
                                    // Set Vx = delay timer value.
                                    // The value of DT is placed into Vx.
                                    registers[x] = delay_timer;
                                }
                                0x0A => {
                                    // Fx0A - LD Vx, K
                                    // Wait for a key press, store the value of the key in Vx.
                                    // All execution stops until a key is pressed, then the value of that key is stored in Vx.
                                    if waiting_for_key.is_none() {
                                        paused = true;
                                        waiting_for_key = Some(x);
                                    }
                                }
                                0x15 => {
                                    // Fx15 - LD DT, Vx
                                    // Set delay timer = Vx.
                                    // DT is set equal to the value of Vx.
                                    delay_timer = registers[x];
                                }
                                0x18 => {
                                    // Fx18 - LD ST, Vx
                                    // Set sound timer = Vx.
                                    // ST is set equal to the value of Vx.
                                    sound_timer = registers[x];
                                }
                                0x1E => {
                                    // Fx1E - ADD I, Vx
                                    // Set I = I + Vx.
                                    // The values of I and Vx are added, and the results are stored in I.
                                    register_i += registers[x] as u16;
                                }
                                0x29 => {
                                    // Fx29 - LD F, Vx
                                    // Set I = location of sprite for digit Vx.
                                    // The value of I is set to the location for the hexadecimal sprite corresponding to the value of Vx.
                                    register_i = (registers[x] * 5) as u16;
                                }
                                0x33 => {
                                    // Fx33 - LD B, Vx
                                    // Store BCD representation of Vx in memory locations I, I+1, and I+2.
                                    // The interpreter takes the decimal value of Vx, and places the hundreds digit in memory at location in I, the tens digit at location I+1, and the ones digit at location I+2.
                                    memory[register_i as usize] = registers[x] / 100;
                                    memory[register_i as usize + 1] = (registers[x] / 10) % 10;
                                    memory[register_i as usize + 2] = registers[x] % 10;
                                }
                                0x55 => {
                                    // Fx55 - LD [I], Vx
                                    // Store registers V0 through Vx in memory starting at location I.
                                    // The interpreter copies the values of registers V0 through Vx into memory, starting at the address in I.
                                    for i in 0..=x {
                                        memory[register_i as usize + i] = registers[i];
                                    }
                                }
                                0x65 => {
                                    // Fx65 - LD Vx, [I]
                                    // Read registers V0 through Vx from memory starting at location I.
                                    // The interpreter reads values from memory starting at location I into registers V0 through Vx.
                                    for i in 0..=x {
                                        registers[i] = memory[register_i as usize + i];
                                    }
                                }
                                _ => unreachable!("Unknown opcode: {:#06X}", opcode),
                            },
                            _ => unreachable!("Unknown opcode: {:#06X}", opcode),
                        }

                        pc += 2;
                    }
                }
                WindowEvent::KeyboardInput {
                    device_id,
                    event,
                    is_synthetic,
                } => {
                    if is_synthetic {
                        return;
                    }

                    if let PhysicalKey::Code(key_code) = event.physical_key {
                        if KeyCode::Space == key_code && event.state.is_pressed() {
                            paused = !paused;
                        }

                        if let Some(key_index) = get_key_index(key_code) {
                            if event.state.is_pressed() {
                                pressed_keys[key_index] = true;
                                if let Some(waiting_x) = waiting_for_key {
                                    registers[waiting_x] = key_index as u8;
                                    paused = false;
                                    waiting_for_key = None;
                                }
                            } else {
                                pressed_keys[key_index] = false;
                            }
                        }
                    }
                }
                WindowEvent::Resized(new_size) => {
                    surface_configured = true;
                    renderer.resize(new_size);
                    window.request_redraw();
                }
                WindowEvent::CloseRequested => target.exit(),
                _ => {}
            };
        }
    })?;

    Ok(())
}

fn get_key_index(key_code: KeyCode) -> Option<usize> {
    /*
        1 2 3 4
        Q W E R
        A S D F
        Z X C V
    */
    const KEY_MAP: [KeyCode; 16] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::KeyQ,
        KeyCode::KeyW,
        KeyCode::KeyE,
        KeyCode::KeyR,
        KeyCode::KeyA,
        KeyCode::KeyS,
        KeyCode::KeyD,
        KeyCode::KeyF,
        KeyCode::KeyZ,
        KeyCode::KeyX,
        KeyCode::KeyC,
        KeyCode::KeyV,
    ];
    KEY_MAP.iter().position(|&k| k == key_code)
}
