use std::env;
use std::fmt;
use std::io::{self, IsTerminal};
use std::sync::OnceLock;

pub struct Styled<T> {
    value: T,
    code: &'static str,
}

impl<T: fmt::Display> Styled<T> {
    fn paint(&self, f: &mut fmt::Formatter<'_>, enabled: bool) -> fmt::Result {
        if !enabled {
            return self.value.fmt(f);
        }
        write!(f, "\x1b[{}m", self.code)?;
        self.value.fmt(f)?;
        f.write_str("\x1b[0m")
    }
}

impl<T: fmt::Display> fmt::Display for Styled<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.paint(f, enabled())
    }
}

pub trait Stylize: fmt::Display + Sized {
    fn green(self) -> Styled<Self> {
        Styled {
            value: self,
            code: "32",
        }
    }
}

impl<T: fmt::Display> Stylize for T {}

fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Forced<'a, T>(&'a Styled<T>);

    impl<T: fmt::Display> fmt::Display for Forced<'_, T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.0.paint(f, true)
        }
    }

    #[test]
    fn wraps_value_in_escape_codes() {
        assert_eq!(format!("{}", Forced(&"CPU".green())), "\x1b[32mCPU\x1b[0m");
    }

    #[test]
    fn applies_width_inside_escape_codes() {
        assert_eq!(
            format!("{:>5}", Forced(&"ab".green())),
            "\x1b[32m   ab\x1b[0m"
        );
    }

    #[test]
    fn prints_plain_value_when_disabled() {
        assert_eq!(format!("{:>5}", "ab".green()), "   ab");
    }
}
