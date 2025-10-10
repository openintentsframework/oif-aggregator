# Changelog

All notable changes to the OIF Aggregator UI will be documented in this file.

## [Unreleased]

### Added
- **Light/Dark Theme Switcher** (2025-10-10): Toggle between light and dark themes
  - **Theme toggle button**: Located in top-right corner of header (☀️/🌙 icon)
  - **Persistent preference**: Theme choice saved to localStorage
  - **System preference**: Defaults to system theme preference on first visit
  - **Comprehensive styling**: All components styled for both themes
  - **High contrast in light mode**: 
    - Darker text on light backgrounds
    - Stronger borders (2px) on inputs and cards
    - Improved readability with slate-900 text on white backgrounds
    - Enhanced placeholder and label contrast
  - **Smooth transitions**: All theme changes animate smoothly
  - **Context-based**: Uses React Context for global theme state
- **Health Status Widget** (2025-10-10): Real-time system health monitoring widget
  - **Bottom-right corner placement**: Unobtrusive, always-accessible widget
  - **Expandable/collapsible**: Click to toggle between compact and detailed view
  - **Auto-refresh**: Polls health endpoint every 30 seconds
  - **Status indicator**: Color-coded status with animated pulse (green/yellow/red)
  - **Detailed statistics**:
    - System status and version
    - Total, active, inactive solver counts
    - Healthy vs. unhealthy solver counts
    - Storage backend and health status
  - **Manual refresh**: Button to immediately fetch latest health data
  - **Error handling**: Shows connection issues clearly
- **Solvers Information Pages** (2025-10-10): New dedicated pages for exploring available solvers
  - **Solvers List Page**: Table view of all solvers with status, coverage, and metadata
    - Status indicators (active, inactive, circuit-breaker-open)
    - Asset/route type badges
    - Chain and asset coverage counts
    - Last seen timestamps
    - Quick navigation to solver details
  - **Solver Detail Page**: Comprehensive view of individual solver information
    - Full metadata (adapter ID, endpoint, created/last seen dates)
    - Asset type and source information
    - For asset-based solvers: Complete list of supported assets grouped by chain
    - For route-based solvers: All supported routes grouped by origin chain
    - Chain name resolution for better readability
  - **Navigation**: New tab-based navigation in header to switch between Quotes and Solvers views
  - **API Integration**: New `solverApi` service with `getSolvers()` and `getSolverById()` methods
  - **Type Definitions**: Added `SolverResponse`, `SolversListResponse`, `SolverStatus`, `AssetInfo`, `RouteInfo`, `SupportedAssetsResponse`

### Changed
- **Default Port** (2025-10-10): Changed development server port from 3000 to 5173 (Vite default)
  - Updated `vite.config.ts` to use port 5173
  - UI now accessible at `http://localhost:5173` instead of `http://localhost:3000`
  - Avoids conflicts with other common development servers on port 3000

### Added (continued)
- **Quote Validity Countdown Timer** (2025-10-09): Real-time countdown showing quote expiration
  - Live timer updates every second showing MM:SS remaining
  - Color-coded warnings (green > 30s, yellow < 30s, red when expired)
  - Automatic disabling of signing and submission when quote expires
  - Clear "EXPIRED" status and instructions to request new quote
  - Warning message when quote is about to expire (< 30 seconds)
  - Prevents submission of expired quotes

### Fixed
- **Route Validation Logic** (2025-10-09): Fixed false "no solvers support this route" warnings
  - Enhanced `getSolversForRoute()` to check both route-based and asset-based solvers
  - Route-based solvers: Check explicit route definitions
  - Asset-based solvers: Check if both origin and destination assets are in solver's asset list
  - Eliminates false negatives where asset-based solvers support the route but warning showed anyway
- **Address Comparison Case Sensitivity** (2025-10-09): Fixed address validation to properly handle case-insensitive comparison
  - Use `fromInteropAddress()` instead of `formatInteropAddress()` to extract full address
  - Compare addresses in lowercase while displaying in original case
  - Prevents false mismatch warnings due to mixed-case addresses (checksum format)

### Added (continued)
- **EIP-712 Address Validation** (2025-10-09): Validates that signer address matches quote request user address
  - Extracts original user address from quote preview using `fromInteropAddress()`
  - Case-insensitive comparison of signer address with expected user address
  - Shows warning banner if addresses don't match
  - Shows success banner if addresses match
  - Helps prevent signing errors and invalid orders

### Changed
- **Consistent Page Widths** (2025-10-09): Applied max-width constraints to all pages for better readability
  - Quote request form: `max-w-2xl` (unchanged)
  - Quote results: `max-w-4xl` (wider for comparison)
  - Order submission: `max-w-3xl` (balanced)
  - Order status: `max-w-4xl` (detailed info)
- **Compact Layout for Better Screen Fit** (2025-10-09): Optimized UI for 15.6" laptop screens
  - Reduced vertical spacing throughout the form (mb-6 → mb-4, space-y-6 → space-y-4)
  - Smaller section padding (p-4 → p-3)
  - Compact input fields with reduced padding (py-2 → py-1.5)
  - Smaller text sizes for labels and headers
  - Grid layout for Swap Type and Amount (side by side)
  - Reduced header and footer padding
  - Smaller font sizes for card titles (text-2xl → text-xl, text-lg → text-base)
  - Entire quote form now fits on screen without scrolling on 15.6" laptops
- **Field Order in Quote Form** (2025-10-09): Reordered fields to show Network before Asset in both From and To sections
  - More intuitive flow: Select network first, then choose from available assets on that network
  - Improved asset filtering based on selected network
  - Better UX alignment with user mental model

### Added
- **Auto-Polling Order Status** (2025-10-09): Automatic status updates every 2 seconds while order is in progress
  - Toggle auto-refresh ON/OFF button with visual indicator
  - Automatically stops polling when order reaches final state (executed, finalized, failed, refunded)
  - "Polling every 2s" animated indicator
  - Visual notifications for final order states (success/failure banners)
  - Smart polling only for non-final states (created, pending, executing, settling)
- **EIP-712 Signing** (2025-10-09): Integrated automatic signature generation with support for multiple schemes:
  - Permit2 (`oif-escrow-v0`) with 0x00 prefix
  - TheCompact (`oif-resource-lock-v0`) with ABI-encoded tuple
  - EIP-3009 (`oif-3009-v0`) with 0x01 prefix
- **Quote Signer Utility** (2025-10-09): Complete TypeScript implementation matching Rust backend signing logic
- **Private Key Input** (2025-10-09): Testing-only private key input with security warnings
- **Signer Address Display** (2025-10-09): Shows derived address from private key for verification
- **Signature Display** (2025-10-09): Shows generated EIP-712 signature before order submission
- **RPC Support** (2025-10-09): Optional RPC URL configuration for domain separator verification

### Fixed
- **InteropAddress Format** (2025-10-09): Fixed quote request body format to send InteropAddress as hex-encoded strings instead of JSON objects with byte arrays. This matches the backend's serde implementation. See `INTEROP_ADDRESS_FIX.md` for details.

### Added (continued)
- **Simplified Quote Form** (2025-10-09): Added dropdown-based UI for selecting assets and networks, making it easier to request quotes without manually entering addresses
- **Solver Data Service** (2025-10-09): Automatic fetching and caching of solver and asset data from `/v1/solvers` endpoint
- **Smart Dropdowns** (2025-10-09): Cascading dropdowns that show only compatible destination chains and assets based on the selected origin
- **Advanced Mode Toggle** (2025-10-09): Toggle between simplified dropdowns and advanced manual address input
- **Form Width Constraint** (2025-10-09): Constrained form width to `max-w-2xl` for better readability

### Changed
- **User Address Field** (2025-10-09): Changed from required input to placeholder with note about wallet connection (to be implemented)
- **Form Layout** (2025-10-09): Simplified form to focus on essential fields: amount, from asset/network, to asset/network

## [0.1.0] - Initial Release

### Added
- Basic quote request form with manual address inputs
- Quote results display with solver information
- Order submission workflow
- Order status tracking
- Solver options configuration
- Multi-step wizard UI
- Responsive design with Tailwind CSS
- Type-safe API integration with TypeScript
