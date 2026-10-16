#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Launch {
    Home,
    Settings,
    Background,
}

impl Launch {
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> Self {
        let args: Vec<_> = arguments.into_iter().collect();
        if args.iter().any(|arg| arg.eq_ignore_ascii_case("--settings")) {
            return Self::Settings;
        }
        // Unknown requests never turn a login item or routed callback into Home.
        if args.is_empty() {
            Self::Home
        } else {
            Self::Background
        }
    }
}
