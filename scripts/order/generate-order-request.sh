#!/usr/bin/env bash

set -euo pipefail

# ============================================================================
# OIF Order Request Generator
# ============================================================================
# 
# This script reads configuration from a JSON file and generates signed
# payloads for the OIF (Open Intent Framework) solver.
# 
# The script extracts the digest from quote data, and creates
# a payload with just the signature and quote response.
#
# Usage:
#   ./generate-order-request.sh config.json
#
# Dependencies:
#   - jq (JSON processing)
#   - cast (from Foundry - for signing)
#
# Config JSON format - see config.json
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
    echo "Usage: $0 <config.json>"
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

echo -e "${BLUE}🧱 OIF Order Request Generator${NC}"
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
        echo -e "${YELLOW}Quote ID: $QUOTE_ID${NC}"
    elif jq -e '.quote.quotes[0].orders[0]' "$CONFIG_FILE" >/dev/null 2>&1; then
        # Legacy format - use first quote
        QUOTE_PATH=".quote.quotes[0]"
        QUOTE_RESPONSE=$(jq -c '.quote.quotes[0]' "$CONFIG_FILE")
        QUOTE_ID=$(jq -r '.quote.quotes[0].quoteId // "unknown"' "$CONFIG_FILE")
        echo -e "${YELLOW}Legacy format - Quote ID: $QUOTE_ID${NC}"
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
    
    # Extract only the digest for signing
    DIGEST=$(jq -r "${ORDER_PATH}.message.digest" "$CONFIG_FILE")
    
elif jq -e '.quote.digest' "$CONFIG_FILE" >/dev/null 2>&1; then
    echo -e "${BLUE}📋 Pre-extracted quote data detected${NC}"
    
    # Pre-extracted quote data (legacy support)
    QUOTE_RESPONSE=$(jq -c '.quote' "$CONFIG_FILE")
    DIGEST=$(jq -r '.quote.digest // empty' "$CONFIG_FILE")
    
else
    echo -e "${RED}Missing quote or quote section in config${NC}"
    exit 1
fi

# Validate required fields
if [ -z "$DIGEST" ]; then
    echo -e "${RED}Missing or invalid digest in quote data${NC}"
    exit 1
fi

echo -e "${BLUE}👤 User:${NC} $USER_ADDR"
echo -e "${BLUE}🔎 Quote data extracted:${NC}"
echo "  Digest:       $DIGEST"

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
# BUILD FINAL PAYLOAD
# ============================================================================

PAYLOAD=$(jq -n \
    --arg sig "$PREFIXED_SIGNATURE" \
    --argjson quote_response "$QUOTE_RESPONSE" \
    '{signature:$sig, quoteResponse:$quote_response}')

# ============================================================================
# OUTPUT OR SUBMIT
# ============================================================================

echo
echo -e "${GREEN}✅ Signed payload ready for submission${NC}"
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
