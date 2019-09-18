#![no_std]
#![no_main]

//extern crate panic_semihosting;
extern crate panic_halt;

mod stm32;
mod vga;
use crate::vga::display::VgaDisplay;
use crate::vga::render::VgaDraw;

use rtfm::app;
use numtoa::NumToA;
use stm32f1::stm32f103 as blue_pill;
use embedded_graphics::prelude::*;
use embedded_graphics::fonts::Font6x8;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::pixelcolor::BinaryColor;

#[app(device = stm32f1::stm32f103)]
const APP: () = {
    // Late resorce binding
    static mut GPIOA: blue_pill::GPIOA = ();
    static mut GPIOC: blue_pill::GPIOC = ();
    static mut TIM2: blue_pill::TIM2 = ();
    static mut TIM3: blue_pill::TIM3 = ();
    static mut TIM4: blue_pill::TIM4 = ();

    // VGA
    static mut DISPLAY : VgaDisplay = VgaDisplay {
        pixels : [0; (vga::HSIZE_CHARS * 8 * vga::VSIZE_CHARS) as usize],
        attributes : [0; (vga::HSIZE_CHARS * vga::VSIZE_CHARS) as usize],
        attribute_definitions : [0; 320]
    };
    static mut VGA_DRAW : VgaDraw = VgaDraw::new();

    #[init (resources = [DISPLAY, VGA_DRAW])]
    fn init() -> init::LateResources {
        // Configure PLL and flash
        stm32::configure_clocks(&device.RCC, &device.FLASH);

        // Configures the system timer to trigger a SysTick exception every second
        //stm32::configure_systick(&mut core.SYST, 72_000); // period = 1ms

        // Built-in LED is on GPIOC, pin 13
        device.RCC.apb2enr.modify(|_r, w| w.iopcen().set_bit());
        device.GPIOC.crh.modify(|_r, w| { w
            .mode13().output50()
            .cnf13().push_pull()
        });

        // This is used to display 64 colors
        for i in 0..64 {
            for j in 0..4 {
                resources.DISPLAY.attribute_definitions[i >> 2 + 64 + j] = i as u8;
            }
        }

        // Initialize VGA
        resources.DISPLAY.init_default_attribute(0x10, 0x3F);
        resources.VGA_DRAW.init(&resources.DISPLAY);
        vga::render::init_vga(&device);

        init::LateResources { 
            GPIOA: device.GPIOA,
            GPIOC: device.GPIOC,
            TIM2: device.TIM2,
            TIM3: device.TIM3,
            TIM4: device.TIM4
        }
    }

    #[idle (resources = [TIM2, TIM4, DISPLAY, GPIOC])]
    fn idle() -> ! {
        resources.DISPLAY.draw(
            Rectangle::new(Point::new(2, 2), Point::new(vga::HSIZE_CHARS as i32 * 8 - 3, vga::VSIZE_CHARS as i32 * 8 - 3)).stroke(Some(BinaryColor::On))
        );
        resources.DISPLAY.draw(
            Rectangle::new(Point::new(4, 4), Point::new(vga::HSIZE_CHARS as i32 * 8 - 5, vga::VSIZE_CHARS as i32 * 8 - 5)).stroke(Some(BinaryColor::On))
        );
        for i in 0..64 {
            let mut buffer = [0u8; 20];
            let color = i.numtoa_str(2, &mut buffer);
            resources.DISPLAY.draw(
                Font6x8::render_str(color)
                .stroke(Some(BinaryColor::On))
                .translate(Point::new(16 + (i % 6) * 56, 41 + (i / 6) * 16))
            );
            for j in 0..5 {
                for y in 0..2 {
                    resources.DISPLAY.pixels[((6 + (i / 6) * 2 + y * 8) * vga::HSIZE_CHARS as i32 + 1 + (i % 6) * 7 + j) as usize] = 0x10; // light blue
                }
                for y in 3..8 {
                    resources.DISPLAY.pixels[((6 + (i / 6) * 2 + y * 8) * vga::HSIZE_CHARS as i32 + 1 + (i % 6) * 7 + j) as usize] = i as u8;
                }
                resources.DISPLAY.attributes[((6 + (i / 6) * 2) * vga::HSIZE_CHARS as i32 + 2 + (i % 6) * 7 + j) as usize] = 1;
            }
        }

        loop {
        }
    }

    #[interrupt (priority = 16, resources = [TIM4, VGA_DRAW])]
    fn TIM4() 
    {
        // Acknowledge IRQ
        resources.TIM4.sr.modify(|_, w| w.cc4if().clear_bit());

        resources.VGA_DRAW.on_vsync();
    }

    #[interrupt (priority = 16, resources = [TIM3, VGA_DRAW])]
    fn TIM3() 
    {
        // Acknowledge IRQ
        resources.TIM3.sr.modify(|_, w| w.cc2if().clear_bit());

        resources.VGA_DRAW.on_hsync();
    }

    #[interrupt (priority = 15, resources = [TIM2])]
    fn TIM2() 
    {
        // Acknowledge IRQ
        resources.TIM2.sr.modify(|_, w| w.cc2if().clear_bit());

        // Idle the CPU until an interrupt arrives
        cortex_m::asm::wfi();
    }
};
