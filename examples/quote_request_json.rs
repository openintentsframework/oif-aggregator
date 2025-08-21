//! Quote Request JSON Generator
//!
//! A simple script to generate quote request JSON for testing.
//! Just modify the constants below and run!
//!
//! Usage:
//!   cargo run --example quote_request_json

use oif_aggregator::models::InteropAddress;

// 🔧 MODIFY THESE VALUES FOR YOUR TESTS
// Simple constants that are easy to change
const USER_ADDRESS: &str = "0x70997970c51812dc3a010c7d01b50e0d17dc79c8";
const INPUT_CHAIN_ID: u64 = 31337; // Local testnet
const INPUT_ASSET: &str = "0x5fbdb2315678afecb367f032d93f642f64180aa3"; // Asset on input chain
const INPUT_AMOUNT: f64 = 1.0; // 1 token
const INPUT_DECIMALS: u8 = 18;

const OUTPUT_CHAIN_ID: u64 = 31338; // Cross-chain destination
const OUTPUT_ASSET: &str = "0x5fbdb2315678afecb367f032d93f642f64180aa3"; // Same asset on output chain
const OUTPUT_AMOUNT: f64 = 1.0; // 1 token
const OUTPUT_DECIMALS: u8 = 18;

const RECEIVER_ADDRESS: Option<&str> = Some("0x3c44cdddb6a900fa2b585dd299e03d12fa4293bc"); // Cross-chain receiver
const VALID_FOR_SECONDS: u64 = 600; // 10 minutes

fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Create addresses using the new CAIP_EIP155_ALT format
	let user_addr = InteropAddress::from_chain_and_address(INPUT_CHAIN_ID, USER_ADDRESS)?;
	let input_addr = InteropAddress::from_chain_and_address(INPUT_CHAIN_ID, INPUT_ASSET)?;
	let output_addr = InteropAddress::from_chain_and_address(OUTPUT_CHAIN_ID, OUTPUT_ASSET)?;
	let receiver_addr = match RECEIVER_ADDRESS {
		Some(addr) => InteropAddress::from_chain_and_address(OUTPUT_CHAIN_ID, addr)?,
		None => user_addr.clone(),
	};

	// Convert amounts to wei/smallest unit
	let input_wei = (INPUT_AMOUNT * 10_f64.powi(INPUT_DECIMALS as i32)) as u64;
	let output_wei = (OUTPUT_AMOUNT * 10_f64.powi(OUTPUT_DECIMALS as i32)) as u64;

	// Create the quote request
	let quote_request = create_quote_request(
		&user_addr,
		&input_addr,
		input_wei,
		&output_addr,
		output_wei,
		&receiver_addr,
		VALID_FOR_SECONDS,
	);

	// Output formatted JSON
	println!("{}", quote_request);

	// Print summary for users
	eprintln!("\n🔧 Configuration Summary:");
	eprintln!("User: {} (chain {})", USER_ADDRESS, INPUT_CHAIN_ID);
	eprintln!("Input: {} tokens → {} wei", INPUT_AMOUNT, input_wei);
	eprintln!("Output: {} tokens → {} wei", OUTPUT_AMOUNT, output_wei);
	eprintln!("Cross-chain: {} → {}", INPUT_CHAIN_ID, OUTPUT_CHAIN_ID);
	eprintln!("Valid for: {} seconds", VALID_FOR_SECONDS);

	Ok(())
}

fn create_quote_request(
	user: &InteropAddress,
	input_asset: &InteropAddress,
	input_amount: u64,
	output_asset: &InteropAddress,
	output_amount: u64,
	receiver: &InteropAddress,
	valid_until: u64,
) -> String {
	format!(
		r#"{{
  "user": "{}",
  "availableInputs": [
    {{
      "user": "{}",
      "asset": "{}",
      "amount": "{}"
    }}
  ],
  "requestedOutputs": [
    {{
      "receiver": "{}",
      "asset": "{}",
      "amount": "{}"
    }}
  ],
  "preference": "speed",
  "minValidUntil": {}
}}"#,
		user.to_hex(),
		user.to_hex(),
		input_asset.to_hex(),
		input_amount,
		receiver.to_hex(),
		output_asset.to_hex(),
		output_amount,
		valid_until
	)
}
