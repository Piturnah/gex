#[cfg(debug_assertions)]
use std::sync::Mutex;

#[cfg(debug_assertions)]
pub struct DebugBuffer {
    pub buf: String,
    pub prev_dimensions: (usize, usize),
}

#[cfg(debug_assertions)]
pub static DBG_BUFFER: Mutex<DebugBuffer> = Mutex::new(DebugBuffer {
    buf: String::new(),
    prev_dimensions: (0, 0),
});

/// Use this macro to debug print things so that they are actually readable while gex is running.
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            if let Ok(mut buf) = $crate::debug::DBG_BUFFER.lock() {
                buf.buf.push_str(&format!($($arg)*));
                buf.buf.push('\n');
            }
        }
    }
}

#[macro_export]
macro_rules! debug_draw {
    () => {
        #[cfg(debug_assertions)]
        {
            if let Ok(mut buf) = $crate::debug::DBG_BUFFER.lock() {
                let $crate::debug::DebugBuffer {
                    ref mut buf,
                    ref mut prev_dimensions,
                } = *buf;

                let (term_width, _) = ::crossterm::terminal::size().unwrap();
                // Clear the previous debug info.
                (0..=prev_dimensions.1).for_each(|i| {
                    print!(
                        "{}{}",
                        ::crossterm::cursor::MoveTo(
                            term_width - prev_dimensions.0 as u16,
                            i as u16
                        ),
                        $crate::render::Clear(::crossterm::terminal::ClearType::UntilNewLine),
                    )
                });

                if !buf.is_empty() {
                    let max_width = buf.lines().map(|l| l.len()).max().expect("!buf.is_empty");
                    let count_lines = buf.lines().count();

                    let cr = term_width - 3 - max_width as u16;
                    buf.lines().enumerate().for_each(|(idx, l)| {
                        println!(
                            "{}\u{2502} {l} ",
                            ::crossterm::cursor::MoveTo(cr, idx as u16)
                        );
                    });
                    print!(
                        "{}\u{2514}{empty:\u{2500}^width$}",
                        ::crossterm::cursor::MoveTo(cr, count_lines as u16),
                        empty = "",
                        width = max_width + 2,
                    );
                    ::std::io::stdout().flush().unwrap();
                    buf.clear();

                    *prev_dimensions = (max_width + 3, count_lines + 1);
                }
            }
        }
    };
}
