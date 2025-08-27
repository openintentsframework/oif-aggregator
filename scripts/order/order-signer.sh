#!/usr/bin/env bash

set -euo pipefail

# ============================================================================
# OIF Order Signer
# ============================================================================
# 
# This script reads configuration from a JSON file and generates signed order
# payloads for the OIF (Open Intent Framework) solver.
#
# Usage:
#   ./order-signer.sh config.json [--dry-run]
#
# Dependencies:
#   - jq (JSON processing)
#   - cast (from Foundry - for signing and ABI encoding)
#   - curl (for HTTP requests, only if submitting)
#
# Config JSON format - see example_config.json
#
# ============================================================================

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Functions
need_cmd() { 
    command -v "$1" >/dev/null 2>&1 || { 
        echo -e "${RED}$1 is required but not found${NC}"; exit 1; 
    } 
}

usage() {
    echo "Usage: $0 <config.json> [options]"
    echo ""
    echo "Options:"
    echo "  --help                   Show this help"
    echo ""
    echo "Example:"
    echo "  $0 config.json"
}

# ============================================================================
# MAIN SCRIPT
# ============================================================================

echo -e "${BLUE}🧱 Standalone OIF Order Signer${NC}"
echo "=================================="

# Check dependencies
need_cmd jq
need_cmd cast

# Parse arguments
if [ $# -eq 0 ] || [ "${1:-}" = "--help" ]; then
    usage
    exit 0
fi

CONFIG_FILE="$1"
DRY_RUN=false

# Parse additional arguments
shift
while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run)
            DRY_RUN=true
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            usage
            exit 1
            ;;
    esac
    shift
done

# Validate config file
if [ ! -f "$CONFIG_FILE" ]; then
    echo -e "${RED}Config file not found: $CONFIG_FILE${NC}"
    exit 1
fi

echo -e "${YELLOW}Reading config from:${NC} $CONFIG_FILE"

# Validate JSON
if ! jq empty "$CONFIG_FILE" 2>/dev/null; then
    echo -e "${RED}Invalid JSON in config file${NC}"
    exit 1
fi

# ============================================================================
# EXTRACT CONFIGURATION
# ============================================================================

echo -e "${YELLOW}📋 Extracting configuration...${NC}"

# Account info
USER_ADDR=$(jq -r '.account.address // empty' "$CONFIG_FILE")
USER_PRIVATE_KEY=$(jq -r '.account.private_key // empty' "$CONFIG_FILE")

if [ -z "$USER_ADDR" ] || [ -z "$USER_PRIVATE_KEY" ]; then
    echo -e "${RED}Missing account.address or account.private_key in config${NC}"
    exit 1
fi

# Check if we have quote or pre-extracted quote data
if jq -e '.quote' "$CONFIG_FILE" >/dev/null 2>&1; then
    echo -e "${BLUE}📋 Raw quote detected - extracting fields...${NC}"
    
    # Determine if quote is single quote or legacy format
    if jq -e '.quote.orders[0]' "$CONFIG_FILE" >/dev/null 2>&1; then
        # Single quote format
        QUOTE_PATH=".quote"
        QUOTE_RESPONSE=$(jq -c '.quote' "$CONFIG_FILE")
        QUOTE_ID=$(jq -r '.quote.quoteId // "unknown"' "$CONFIG_FILE")
        SOLVER_ID=$(jq -r '.quote.solverId // "unknown"' "$CONFIG_FILE")
        echo -e "${YELLOW}Quote ID: $QUOTE_ID (solver: $SOLVER_ID)${NC}"
    elif jq -e '.quote.quotes[0].orders[0]' "$CONFIG_FILE" >/dev/null 2>&1; then
        # Legacy format - use first quote
        QUOTE_PATH=".quote.quotes[0]"
        QUOTE_RESPONSE=$(jq -c '.quote.quotes[0]' "$CONFIG_FILE")
        QUOTE_ID=$(jq -r '.quote.quotes[0].quoteId // "unknown"' "$CONFIG_FILE")
        SOLVER_ID=$(jq -r '.quote.quotes[0].solverId // "unknown"' "$CONFIG_FILE")
        echo -e "${YELLOW}Legacy format - Quote ID: $QUOTE_ID (solver: $SOLVER_ID)${NC}"
    else
        echo -e "${RED}Invalid quote format${NC}"
        exit 1
    fi
    
    # Build the jq path for order data
    if [ "$QUOTE_PATH" = ".quote" ]; then
        ORDER_PATH=".quote.orders[0]"
    else
        ORDER_PATH=".quote.quotes[0].orders[0]"
    fi
    
    # Extract fields from raw quote
    DIGEST=$(jq -r "${ORDER_PATH}.message.digest" "$CONFIG_FILE")
    NONCE=$(jq -r "${ORDER_PATH}.message.eip712.nonce // \"0\"" "$CONFIG_FILE")
    DEADLINE=$(jq -r "${ORDER_PATH}.message.eip712.deadline // \"1000000000\"" "$CONFIG_FILE")
    EXPIRY=$(jq -r "${ORDER_PATH}.message.eip712.witness.expires" "$CONFIG_FILE")
    ORACLE_ADDRESS=$(jq -r "${ORDER_PATH}.message.eip712.witness.inputOracle" "$CONFIG_FILE")
    ORIGIN_CHAIN_ID=$(jq -r "${ORDER_PATH}.message.eip712.signing.domain.chainId" "$CONFIG_FILE")
    ORIGIN_TOKEN=$(jq -r "${ORDER_PATH}.message.eip712.permitted[0].token" "$CONFIG_FILE")
    AMOUNT=$(jq -r "${ORDER_PATH}.message.eip712.permitted[0].amount" "$CONFIG_FILE")
    DEST_CHAIN_ID=$(jq -r "${ORDER_PATH}.message.eip712.witness.outputs[0].chainId" "$CONFIG_FILE")
    OUTPUT_SETTLER_BYTES32=$(jq -r "${ORDER_PATH}.message.eip712.witness.outputs[0].settler" "$CONFIG_FILE")
    DEST_TOKEN_BYTES32=$(jq -r "${ORDER_PATH}.message.eip712.witness.outputs[0].token" "$CONFIG_FILE")
    RECIPIENT_BYTES32=$(jq -r "${ORDER_PATH}.message.eip712.witness.outputs[0].recipient" "$CONFIG_FILE")
    
elif jq -e '.quote.digest' "$CONFIG_FILE" >/dev/null 2>&1; then
    echo -e "${BLUE}📋 Pre-extracted quote data detected${NC}"
    
    # Pre-extracted quote data (legacy support)
    QUOTE_RESPONSE=$(jq -c '.quote' "$CONFIG_FILE")
    DIGEST=$(jq -r '.quote.digest // empty' "$CONFIG_FILE")
    NONCE=$(jq -r '.quote.nonce // "0"' "$CONFIG_FILE")
    DEADLINE=$(jq -r '.quote.deadline // "1000000000"' "$CONFIG_FILE")
    EXPIRY=$(jq -r '.quote.expiry // empty' "$CONFIG_FILE")
    ORACLE_ADDRESS=$(jq -r '.quote.oracle_address // empty' "$CONFIG_FILE")
    ORIGIN_CHAIN_ID=$(jq -r '.quote.origin_chain_id // empty' "$CONFIG_FILE")
    ORIGIN_TOKEN=$(jq -r '.quote.origin_token // empty' "$CONFIG_FILE")
    AMOUNT=$(jq -r '.quote.amount // empty' "$CONFIG_FILE")
    DEST_CHAIN_ID=$(jq -r '.quote.dest_chain_id // empty' "$CONFIG_FILE")
    OUTPUT_SETTLER_BYTES32=$(jq -r '.quote.output_settler_bytes32 // empty' "$CONFIG_FILE")
    DEST_TOKEN_BYTES32=$(jq -r '.quote.dest_token_bytes32 // empty' "$CONFIG_FILE")
    RECIPIENT_BYTES32=$(jq -r '.quote.recipient_bytes32 // empty' "$CONFIG_FILE")
    
else
    echo -e "${RED}Missing quote or quote section in config${NC}"
    exit 1
fi

# Validate required fields
if [ -z "$DIGEST" ]; then
    echo -e "${RED}Missing or invalid digest in quote data${NC}"
    exit 1
fi

required_fields=("EXPIRY" "ORACLE_ADDRESS" "ORIGIN_CHAIN_ID" "ORIGIN_TOKEN" "AMOUNT" "DEST_CHAIN_ID" "OUTPUT_SETTLER_BYTES32" "DEST_TOKEN_BYTES32" "RECIPIENT_BYTES32")
for field in "${required_fields[@]}"; do
    if [ -z "${!field}" ]; then
        echo -e "${RED}Missing ${field,,} in quote data${NC}"
        exit 1
    fi
done

echo -e "${BLUE}👤 User:${NC} $USER_ADDR"
echo -e "${BLUE}🔎 Quote data extracted:${NC}"
echo "  Digest:       $DIGEST"
echo "  Nonce:        $NONCE"
echo "  Deadline:     $DEADLINE"
echo "  Expiry:       $EXPIRY"
echo "  Oracle:       $ORACLE_ADDRESS"
echo "  Origin CID:   $ORIGIN_CHAIN_ID"
echo "  Origin Token: $ORIGIN_TOKEN"
echo "  Amount:       $AMOUNT"
echo "  Dest CID:     $DEST_CHAIN_ID"
echo "  Output Settler: $OUTPUT_SETTLER_BYTES32"
echo "  Dest Token:   $DEST_TOKEN_BYTES32"
echo "  Recipient:    $RECIPIENT_BYTES32"

# ============================================================================
# SIGN DIGEST
# ============================================================================

echo -e "${YELLOW}✍️  Signing digest...${NC}"
SIGNATURE=$(cast wallet sign --no-hash --private-key "$USER_PRIVATE_KEY" "$DIGEST")
if [ -z "$SIGNATURE" ]; then
    echo -e "${RED}Failed to sign digest${NC}"
    exit 1
fi
PREFIXED_SIGNATURE="0x00${SIGNATURE:2}"
echo -e "${GREEN}✅ Signature:${NC} $PREFIXED_SIGNATURE"

# ============================================================================
# ENCODE STANDARD ORDER
# ============================================================================

echo -e "${YELLOW}🧩 Encoding StandardOrder...${NC}"

# StandardOrder ABI type
STANDARD_ORDER_ABI_TYPE='f((address,uint256,uint256,uint32,uint32,address,uint256[2][],(bytes32,bytes32,uint256,bytes32,uint256,bytes32,bytes,bytes)[]))'

ZERO_BYTES32=0x0000000000000000000000000000000000000000000000000000000000000000
FILL_DEADLINE=$DEADLINE

ORDER_DATA=$(cast abi-encode "$STANDARD_ORDER_ABI_TYPE" \
"(${USER_ADDR},${NONCE},${ORIGIN_CHAIN_ID},${EXPIRY},${FILL_DEADLINE},${ORACLE_ADDRESS},[[$ORIGIN_TOKEN,$AMOUNT]],[($ZERO_BYTES32,$OUTPUT_SETTLER_BYTES32,${DEST_CHAIN_ID},$DEST_TOKEN_BYTES32,$AMOUNT,$RECIPIENT_BYTES32,0x,0x)])")

if [ -z "$ORDER_DATA" ]; then
    echo -e "${RED}Failed to encode StandardOrder${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Order encoded${NC}"

# ============================================================================
# BUILD FINAL PAYLOAD
# ============================================================================

PAYLOAD=$(jq -n \
    --arg order "$ORDER_DATA" \
    --arg sponsor "$USER_ADDR" \
    --arg sig "$PREFIXED_SIGNATURE" \
    --argjson quote_response "$QUOTE_RESPONSE" \
    '{order:$order, sponsor:$sponsor, signature:$sig, quoteResponse:$quote_response}')

# ============================================================================
# OUTPUT OR SUBMIT
# ============================================================================

    echo
    echo -e "${GREEN}✅ Signed payload ready for external use${NC}"
    echo
    echo -e "${BLUE}📋 Signed Payload (for Postman/API tools):${NC}"
    echo -e "${YELLOW}=================================================================================${NC}"
    echo "$PAYLOAD" | jq .
    echo -e "${YELLOW}=================================================================================${NC}"
    echo
    echo -e "${BLUE}📤 Ready-to-use curl command:${NC}"
    echo -e "${YELLOW}=================================================================================${NC}"
    echo "curl -X POST http://localhost:4000/v1/orders \\"
    echo "  -H \"Content-Type: application/json\" \\"
    echo "  -d '$PAYLOAD'"
    echo -e "${YELLOW}=================================================================================${NC}"
    echo
    echo -e "${BLUE}📤 API Details:${NC}"
    echo -e "  Method: POST"
    echo -e "  URL: http://localhost:4000/v1/orders"
    echo -e "  Content-Type: application/json"
    echo
    echo -e "${GREEN}🎉 Copy the curl command above or use the payload in your API client${NC}"

