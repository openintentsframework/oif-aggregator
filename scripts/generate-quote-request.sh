#!/usr/bin/env bash

# 🔧 QuoteRequest Generator - Simple Configuration
# Edit the constants below to generate your QuoteRequest JSON

# =================================
# USER CONFIGURATION - EDIT THESE!
# =================================

# Chain IDs and Token Addresses
ORIGIN_CHAIN_ID=11155420
ORIGIN_TOKEN_ADDRESS="0x4200000000000000000000000000000000000006"
DEST_CHAIN_ID=129399
DEST_TOKEN_ADDRESS="0x17B8Ee96E3bcB3b04b3e8334de4524520C51caB4"

# User and Receiver Addresses
USER_ADDRESS="0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
RECEIVER_ADDRESS="$USER_ADDRESS"

# Amount in wei
AMOUNT="100000000000000"  # 0,0001 ETH

# Quote Settings
MIN_VALID_UNTIL=300  # 5 minutes
PREFERENCE="speed"   # Options: price, speed, inputPriority, trustMinimization

# =================================
# SCRIPT LOGIC - DON'T EDIT BELOW
# =================================

set -e

# Function to encode chain ID using minimal big-endian encoding
encode_chain_id() {
    local chain_id=$1
    
    if [[ $chain_id -eq 0 ]]; then
        echo "Error: Chain ID cannot be 0" >&2
        return 1
    fi
    
    # Convert to hex and remove leading zeros
    local hex=$(printf "%016x" $chain_id | sed 's/^0*//')
    if [[ -z "$hex" ]]; then
        hex="0"
    fi
    
    # Ensure even length for proper byte encoding
    if [[ $((${#hex} % 2)) -eq 1 ]]; then
        hex="0$hex"
    fi
    
    echo "$hex"
}

# Function to create InteropAddress (ERC-7930 binary format)
create_interop_address() {
    local chain_id=$1
    local address=$2
    
    # Normalize address to lowercase and remove 0x prefix
    local addr_hex
    if [[ "$address" == 0x* ]] || [[ "$address" == 0X* ]]; then
        addr_hex=$(echo "${address:2}" | tr '[:upper:]' '[:lower:]')
    else
        addr_hex=$(echo "$address" | tr '[:upper:]' '[:lower:]')
    fi
    
    # Validate address length (20 bytes = 40 hex chars)
    if [[ ${#addr_hex} -ne 40 ]]; then
        echo "Error: Invalid Ethereum address length: $address" >&2
        return 1
    fi
    
    # Encode chain ID using minimal big-endian
    local chain_ref_hex=$(encode_chain_id $chain_id)
    local chain_ref_len=$((${#chain_ref_hex} / 2))
    
    # Build binary format:
    # 01 = version (1 byte)
    # 0000 = chain type EIP-155 (2 bytes) 
    # CRL = chain reference length (1 byte)
    # 14 = address length 20 bytes (1 byte)
    # chain_ref_hex = encoded chain ID (CRL bytes)
    # addr_hex = address (20 bytes)
    
    local version="01"
    local chain_type="0000"
    local crl=$(printf "%02x" $chain_ref_len)
    local adl="14"  # 20 bytes = 0x14
    
    echo "0x${version}${chain_type}${crl}${adl}${chain_ref_hex}${addr_hex}"
}

# Show configuration
echo "🔧 Generating QuoteRequest with your configuration:"
echo "  Origin: Chain $ORIGIN_CHAIN_ID, Token $ORIGIN_TOKEN_ADDRESS"
echo "  Destination: Chain $DEST_CHAIN_ID, Token $DEST_TOKEN_ADDRESS"
echo "  User: $USER_ADDRESS"
echo "  Receiver: $RECEIVER_ADDRESS"
echo "  Amount: $AMOUNT wei"
echo "  Min Valid: $MIN_VALID_UNTIL seconds"
echo "  Preference: $PREFERENCE"
echo

# Create InteropAddresses
USER_INTEROP=$(create_interop_address "$ORIGIN_CHAIN_ID" "$USER_ADDRESS")
INPUT_ASSET_INTEROP=$(create_interop_address "$ORIGIN_CHAIN_ID" "$ORIGIN_TOKEN_ADDRESS")
OUTPUT_ASSET_INTEROP=$(create_interop_address "$DEST_CHAIN_ID" "$DEST_TOKEN_ADDRESS")
RECEIVER_INTEROP=$(create_interop_address "$DEST_CHAIN_ID" "$RECEIVER_ADDRESS")

# Generate JSON manually (no dependencies needed!)
cat << EOF
{
  "user": "$USER_INTEROP",
  "availableInputs": [
    {
      "user": "$USER_INTEROP",
      "asset": "$INPUT_ASSET_INTEROP", 
      "amount": "$AMOUNT"
    }
  ],
  "requestedOutputs": [
    {
      "receiver": "$RECEIVER_INTEROP",
      "asset": "$OUTPUT_ASSET_INTEROP",
      "amount": "$AMOUNT"
    }
  ],
  "minValidUntil": $MIN_VALID_UNTIL,
  "preference": "$PREFERENCE"
}
EOF