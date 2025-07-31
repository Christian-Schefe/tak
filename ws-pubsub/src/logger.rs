#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        #[cfg(feature = "dioxus")]
        {
            dioxus::logger::tracing::info!($($arg)*);
        }
        #[cfg(not(feature = "dioxus"))]
        {
            println!($($arg)*);
        }
    }};
}
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        #[cfg(feature = "dioxus")]
        {
            dioxus::logger::tracing::error!($($arg)*);
        }
        #[cfg(not(feature = "dioxus"))]
        {
            eprintln!($($arg)*);
        }
    }};
}
