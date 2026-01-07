# OIF Aggregator Testing UI

A modern, user-friendly testing interface for the OIF Aggregator API. Built with React, TypeScript, Vite, and Tailwind CSS.

## Features

### 🎯 **Smart Dropdown-Based Quote Form**
- **Asset Selection**: Choose from available assets via searchable dropdowns
- **Network Selection**: Select chains with visual indicators
- **Auto-Discovery**: Automatically fetches solver and asset data from API
- **Smart Routing**: Shows only compatible destination chains/assets
- **Route Validation**: Warns if no solvers support selected route
- **Two Modes**: Simple (dropdown) or Advanced (manual input)

### 📊 **Quote Management**
- View and compare quotes from multiple solvers
- Sort by ETA, solver name, or default
- Detailed quote information (inputs, outputs, metadata)
- Aggregation statistics display

### 🔍 **Solver Explorer**
- **Solvers List**: Browse all available solvers with status and coverage info
  - Status indicators (active, inactive, circuit-breaker-open)
  - Asset vs. route-based solver types
  - Chain and asset coverage counts
  - Last seen timestamps
- **Solver Details**: Deep dive into individual solver capabilities
  - Complete supported assets list (asset-based solvers)
  - All supported routes (route-based solvers)
  - Assets grouped by chain for easy navigation
  - Adapter and endpoint information
  - Discovery source (autoDiscovered vs. manual)

### 📝 **Order Flow**
- **EIP-712 Signing**: Automatic signature generation with support for Permit2, TheCompact, and EIP-3009
- Submit orders with cryptographic signatures
- Track order execution status
- **Auto-Polling**: Automatic status updates every 2 seconds while order is in progress
- Toggle auto-refresh ON/OFF
- Manual refresh capability
- Visual notifications for final order states
- Complete order history

### 🩺 **Health Monitoring**
- **Real-time Health Widget**: Always-visible status indicator in bottom-right corner
  - Auto-refreshes every 30 seconds
  - Color-coded status (green/yellow/red)
  - Click to expand for detailed statistics
  - Shows solver counts, storage health, and version
  - Manual refresh capability

### 🎨 **Theme Switcher**
- **Light and Dark themes**: Toggle between light and dark modes
- **Top-right button**: ☀️ (light mode) / 🌙 (dark mode) in header
- **Persistent**: Theme choice saved and restored on next visit
- **System-aware**: Defaults to system preference if not set
- **High contrast**: Optimized readability in both themes

### ⚙️ **Settings Configuration**
- **Aggregator URL Configuration**: Connect to different OIF Aggregator instances
  - Dynamic URL switching without reloading the app
  - Test connection before saving
  - Reset to default URL option
  - Status indicator (default vs. custom URL)
- **Automatic Data Refresh**: Solver and asset data automatically refetches when URL changes
- **Persistent**: Custom URL saved to localStorage

### ⚡ **Performance & UX**
- LocalStorage caching (5-minute TTL)
- Fast subsequent page loads
- Loading states and error handling
- Responsive design (mobile-friendly)
- Modern themes (light & dark) inspired by bridge/swap UIs

## Prerequisites

- Node.js 18+ 
- pnpm or yarn
- **OIF Aggregator server** running on port 4000 (or custom port configured in Settings)
- **One or more solvers** configured and running
  - For demo setup, see the [OIF Solver repository](https://github.com/openintentsframework/oif-solver)
  - Follow the Quick Start guide to run solvers locally
  - Configure the aggregator to connect to your solver instances
  - Ensure solvers are registered and active before requesting quotes

## Getting Started

Before using the UI, you need to have the OIF Aggregator and at least one solver running.

### Complete Setup Workflow

1. **Set up and run solvers** (see [OIF Solver repository](https://github.com/openintentsframework/oif-solver))
   ```bash
   # In the oif-solver directory
   ./oif-demo env up          # Start local chains and deploy contracts
   cargo run --bin solver -- --config config/demo.toml
   ```

2. **Configure and start the OIF Aggregator** with your solver(s)
   - Configure the aggregator to connect to your solver instances
   - Start the aggregator on port 4000 (or your preferred port)
   - Verify solvers are registered: `GET http://localhost:4000/api/api/v1/solvers`

   Note: The OIF Aggregator API uses `/api/v1` prefix for all API endpoints.

3. **Start the UI** (this project)
   - Install dependencies and run the development server (see below)
   - Configure the aggregator URL in Settings if using a custom port

### 1. Install Dependencies

```bash
cd demo
pnpm install
```

### 2. Configure Environment (Optional)

Create a `.env` file if you need to change the default API URL:

```bash
VITE_API_BASE_URL=http://localhost:4000
```

By default, the UI connects to `localhost:4000`. You can also change this at runtime in the Settings page.

### 3. Start Development Server

```bash
pnpm dev
```

The UI will be available at `http://localhost:5173`.

### 4. Generate TypeScript Types (Optional)

To generate TypeScript types from the OpenAPI specification:

```bash
pnpm generate-types
```

This will create `src/types/api.ts` from the OpenAPI spec at `../docs/api/openapi.json`.

## Usage Guide

The UI has two main sections: **Quotes** (for requesting and submitting cross-chain swaps) and **Solvers** (for exploring available solver capabilities).

### Exploring Solvers

Click the **🔍 Solvers** tab in the header to explore available solvers.

#### Solvers List View

View all solvers with key information:
- **Status**: Visual indicators showing if solver is active, inactive, or has circuit breaker open
- **Solver Info**: Name, ID, and description
- **Adapter**: The adapter type being used
- **Coverage**: Number of supported assets/routes and chains
- **Type**: Asset-based (any-to-any) or Route-based (specific paths)
- **Last Seen**: When the solver was last active

Click **View Details →** on any solver to see comprehensive information.

#### Solver Detail View

Detailed information about a specific solver:
- **Overview**: Adapter ID, endpoint, created date, last seen
- **Asset Type**: Whether it's asset-based or route-based
- **For Asset-Based Solvers**: Complete list of supported assets grouped by chain
  - Asset symbol, name, address, and decimals
  - Can transfer between any assets in the list
- **For Route-Based Solvers**: All supported routes grouped by origin chain
  - Specific origin → destination pairs
  - Token addresses and symbols for each route

Use the **← Back to Solvers** button to return to the list.

---

### Health Status Widget

A real-time health monitoring widget is always visible in the **bottom-right corner** of the screen.

#### Collapsed View
- **Status indicator**: Pulsing dot showing system health (green/yellow/red)
- **Status text**: System status (HEALTHY, DEGRADED, UNHEALTHY)
- **Solver count**: Quick view of active/total solvers (e.g., "2/3 solvers")
- Click to expand for detailed statistics

#### Expanded View
Shows comprehensive system information:
- **Version**: Current OIF Aggregator version
- **Solvers**: Total, active, inactive, healthy, and unhealthy counts
- **Storage**: Backend type and health status
- **Refresh button**: Manually fetch latest health data

The widget automatically refreshes every 30 seconds to keep data current.

---

### Settings

Click the **⚙️ Settings** tab in the header to configure the UI.

#### Aggregator URL Configuration

The Settings page allows you to connect to different OIF Aggregator instances:

1. **Current Status**: Shows whether you're using the default or a custom URL
2. **Default URL**: Displays the default aggregator URL (from environment variables)
3. **URL Input**: Enter the base URL of your OIF Aggregator instance
   - Example: `http://localhost:3000` or `https://aggregator.example.com`
   - Don't include trailing slashes
4. **Test Connection**: Click to verify the URL is valid before saving
   - Makes a request to the `/health` endpoint
   - Shows success with version info or error message
5. **Save**: Saves the URL and automatically refetches all solver and asset data from the new endpoint
6. **Reset to Default**: Reverts to the default URL and refetches data

The custom URL is saved to localStorage and persists across browser sessions. All API requests will use the configured URL, allowing you to easily switch between different aggregator instances (local, staging, production).

---

### Quote Flow

The quotes section provides two modes: **Simple Mode** (dropdown-based, recommended) and **Advanced Mode** (manual address input). Click the **🔄 Quotes** tab to access this flow.

### Step 1: Request Quote

#### Simple Mode (Default, Recommended)

1. **Enter Amount**: Input the swap amount in token units
2. **From Section**:
   - Select **Network** first (e.g., Ethereum, Base, Optimism)
   - Select **Asset** from available assets on that network
3. **To Section**:
   - Select **Network** (filtered by compatibility with your source asset)
   - Select **Asset** from available assets on the destination network
4. **Enter User Address**: Your Ethereum address (0x...)
5. Click **Get Quotes**

The UI automatically:
- Shows only assets available on the selected network
- Filters compatible destination networks based on your source asset
- Warns if no solvers support the selected route
- Validates the route before sending the request

#### Advanced Mode (Manual Input)

1. **Enter Input Details**:
   - User address (Ethereum format: `0x...`)
   - User chain ID (e.g., `1` for Ethereum mainnet)
   - Asset address to swap from
   - Amount (in wei or token's smallest unit)

2. **Enter Output Details**:
   - Receiver address
   - Output chain ID
   - Asset address to receive
   - Desired amount (optional for exact-input swaps)

3. **Select Swap Type**:
   - **Exact Input**: Fixed input amount, variable output
   - **Exact Output**: Variable input amount, fixed output

4. **Choose Supported Order Types**:
   - Select which order types your application supports
   - Default: `oif-escrow-v0`

5. **Configure Solver Options** (Optional):
   - Timeout settings
   - Minimum quotes required
   - Solver selection strategy (all/sampled/priority)
   - Include/exclude specific solvers

6. Click **Get Quotes**

### Step 2: Select Quote

1. Review quotes from different solvers
2. Compare:
   - Input/output amounts
   - ETA (estimated time)
   - Solver reliability
   - Failure handling options
3. Sort by ETA or solver name
4. Click **Select This Quote** on your preferred option

### Step 3: Sign the Quote

1. **Monitor Quote Validity**
   - A countdown timer shows how long the quote remains valid
   - 🟢 Green: More than 30 seconds remaining
   - 🟡 Yellow: Less than 30 seconds remaining (hurry!)
   - 🔴 Red: Quote expired (must request new quote)
2. **Enter your private key** (for testing only!)
   - ⚠️ WARNING: Never share your private key! Use wallet integration in production.
3. Click **Sign Quote with EIP-712**
   - The UI will automatically:
     - Check if quote is still valid (disabled if expired)
     - Derive your signer address
     - **Validate** the signer address matches the quote request user address
     - Construct the EIP-712 typed data
     - Generate the signature with the appropriate scheme (Permit2, TheCompact, or EIP-3009)
     - Display the generated signature
4. **Address Validation Feedback**:
   - ✓ Green banner: Signer address displayed
   - ✓ Blue banner: Address matches quote request (correct)
   - ⚠️ Yellow banner: Address mismatch warning (check your private key!)
5. Review the generated signature

### Step 4: Submit Order

1. Review selected quote details
2. The signature is automatically filled from Step 3
3. Optionally add custom metadata (JSON format)
4. Click **Submit Signed Order**

### Step 5: Track Order Status

1. View order status and details
2. **Auto-Refresh**: The status automatically polls every 2 seconds while the order is in progress
   - Shows "• Polling every 2s" indicator
   - Toggle auto-refresh ON/OFF with the button
   - Automatically stops when order reaches a final state
3. **Manual Refresh**: Click **Refresh Status** to update immediately
4. Monitor execution progress through status badges
5. Click **Start Over** to create a new quote request

## Project Structure

```
ui/
├── src/
│   ├── components/          # React components
│   │   ├── QuoteRequestForm.tsx
│   │   ├── SolverOptions.tsx
│   │   ├── QuoteResults.tsx
│   │   ├── QuoteCard.tsx
│   │   ├── OrderSubmission.tsx
│   │   └── OrderStatus.tsx
│   ├── services/           # API client
│   │   └── api.ts
│   ├── types/             # TypeScript types
│   │   └── api.ts
│   ├── utils/             # Utility functions
│   │   └── interopAddress.ts
│   ├── App.tsx            # Main app component
│   ├── main.tsx           # Entry point
│   └── index.css          # Global styles
├── public/
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.js
└── README.md
```

## API Integration

The UI integrates with the following OIF Aggregator endpoints:

- `POST /api/v1/quotes` - Request quotes from solvers
- `POST /api/v1/orders` - Submit an order for execution
- `GET /api/v1/orders/{id}` - Get order status by ID
- `GET /health` - Health check

## Development

### Available Scripts

- `pnpm dev` - Start development server
- `pnpm build` - Build for production
- `pnpm preview` - Preview production build
- `pnpm generate-types` - Generate TypeScript types from OpenAPI spec

### Building for Production

```bash
pnpm build
```

The built files will be in the `dist/` directory.

### Serving Built Files

You can serve the built files with any static file server:

```bash
pnpm preview
```

Or integrate with the Rust server by serving the `dist/` directory.

## InteropAddress Format

The UI automatically converts between standard Ethereum addresses and the ERC-7930 InteropAddress format:

**Input (User-Friendly)**:
```
Address: 0xA0b86a33E6441E7C81F7C93451777f5F4dE78e86
Chain ID: 1
```

**Converted to (ERC-7930)**:
```json
{
  "version": 1,
  "chain_type": [0, 0, 0, 60],
  "chain_reference": [1],
  "address": [160, 184, 106, 51, ...]
}
```

## Testing Tips

### Example Test Data

**Ethereum Mainnet WETH to Optimism USDC**:

Input:
```
User Address: 0x70997970C51812dc3A010C7d01b50e0d17dc79C8
Chain ID: 1
Asset: 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2
Amount: 1000000000000000000 (1 WETH)
```

Output:
```
Receiver: 0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC
Chain ID: 10
Asset: 0x7F5c764cBc14f9669B88837ca1490cCa17c31607
Amount: 1000000 (1 USDC)
```

### Mock Signature

For testing without actual wallet signing:
```
0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12
```

## Troubleshooting

### Server Connection Issues

If you see "No response from server":

1. Ensure the OIF Aggregator server is running
2. Check the server is accessible at `http://localhost:4000`
3. Verify CORS is enabled on the server
4. Check browser console for detailed errors
5. Try using the Settings page to test the connection

### No Solvers Available

If you see "No solvers currently support this route" or empty solver lists:

1. **Check solver status**: Navigate to the **🔍 Solvers** tab to see registered solvers
2. **Verify solver is running**: Ensure your solver process is active (see [OIF Solver Quick Start](https://github.com/openintentsframework/oif-solver))
3. **Check solver registration**: The solver should appear in `GET /api/v1/solvers` endpoint
4. **Review solver configuration**: Ensure the solver's supported assets/routes match your quote request
5. **Check aggregator logs**: Look for solver registration and health check messages
6. **Restart solver**: Sometimes re-registering helps: stop and restart the solver process

### Invalid Address Errors

- Ethereum addresses must be 42 characters (0x + 40 hex digits)
- Chain IDs must be positive integers
- Amounts should be in the token's smallest unit (wei for ETH)

### Quote Request Fails

- Ensure at least one input and one output are specified
- Check that supported order types are selected
- Verify solver options are valid (positive numbers)

### InteropAddress Format

The UI automatically converts Ethereum addresses and chain IDs to ERC-7930 InteropAddress format (hex-encoded strings). If you're debugging API requests:

- Addresses are sent as hex strings like `0x00010000010114d8da6bf26964af9d7eed9e03e53415d37aa96045`, not as JSON objects
- Format per EIP-7930: `[version][chain_type][chain_ref_len][chain_reference][addr_len][address]`
- **Important**: ChainRef comes BEFORE AddrLen (not after)
- See `EIP-7930-FIX-SUMMARY.md` for detailed format specification

### Signing Errors

If you encounter issues with EIP-712 signing:

- **"Invalid primary type"**: The order payload doesn't include the required EIP-712 types
- **"Failed to fetch domain separator"**: RPC issue or contract doesn't exist (signing still works without this)
- **Invalid private key format**: Ensure it's a hex string (with or without 0x prefix)
- **Signature mismatch**: Each order type has specific encoding (Permit2, TheCompact, EIP-3009)

### Quote Expired

If you see "Quote Expired" or the timer shows "EXPIRED":

- **Cause**: Too much time has elapsed since the quote was generated
- **Solution**: 
  1. Click "← Back to Quotes" button
  2. Click "Back to Request Form" 
  3. Submit a new quote request
- **Why it happens**: Quotes have limited validity (typically 5-15 minutes) to ensure pricing accuracy
- **Tip**: Complete signing and submission quickly when timer shows < 30 seconds

### Address Mismatch Warning

If you see a yellow "Address Mismatch Warning" banner:

- **Cause**: The private key you're using doesn't correspond to the user address in the quote request
- **Solution**: 
  1. Verify you entered the correct private key
  2. Check that the user address in the quote request matches your intended account
  3. If intentionally signing with a different account, you can proceed but the order may fail
- **Why it matters**: Orders must be signed by the user address specified in the quote request
- **Note**: The UI automatically extracts and validates addresses to prevent errors

## Contributing

This UI is part of the OIF Aggregator project. For contributions:

1. Follow the existing code style
2. Use TypeScript for type safety
3. Test all form interactions

## License

MIT License - see the main project LICENSE file.

