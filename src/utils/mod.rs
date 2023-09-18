use std::io::Write;
use termcolor::WriteColor;

pub struct StyledStr {
    messages: Vec<(Option<Style>, String)>,
}

impl StyledStr {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn push_str(&mut self, style: Option<Style>, msg: String) {
        if !msg.is_empty() {
            self.messages.push((style, msg));
        }
    }

    pub fn print_err(&self) -> std::io::Result<()> {
        let bufwtr = termcolor::BufferWriter::stderr(termcolor::ColorChoice::Always);
        let mut buffer = bufwtr.buffer();

        for (style, message) in &self.messages {
            let mut color = termcolor::ColorSpec::new();
            match style {
                Some(Style::Success) => {
                    color.set_fg(Some(termcolor::Color::Green));
                }
                Some(Style::Warning) => {
                    color.set_fg(Some(termcolor::Color::Yellow));
                }
                Some(Style::Error) => {
                    color.set_fg(Some(termcolor::Color::Red));
                    color.set_bold(true);
                }
                None => {}
            }

            buffer.set_color(&color)?;
            write!(buffer, "{message}")?;
        }

        write!(buffer, "\r\n")?;
        bufwtr.print(&buffer)?;

        Ok(())
    }

    pub fn print_data_err(&self) -> ! {
        self.print_err().unwrap();

        // DATAERR
        std::process::exit(65)
    }

    pub fn print_success(&self) -> ! {
        self.print_err().unwrap();

        // SUCCESS
        std::process::exit(0)
    }
}

pub enum Style {
    Success,
    Warning,
    Error,
}
