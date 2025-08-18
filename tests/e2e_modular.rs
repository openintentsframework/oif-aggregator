//! Modular end-to-end tests
//!
//! This replaces the monolithic e2e_test.rs with a more organized structure

mod e2e;
mod mocks;

// Import all test modules
mod health_tests {
	include!("e2e/health_tests.rs");
}

mod middleware_tests {
	include!("e2e/middleware_tests.rs");
}

mod quotes_tests {
	include!("e2e/quotes_tests.rs");
}

mod orders_tests {
	include!("e2e/orders_tests.rs");
}

mod solver_options_tests {
	include!("e2e/solver_options_tests.rs");
}
