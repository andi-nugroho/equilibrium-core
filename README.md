# Equilibrium-Core

A high-performance AMM implementing weighted asset exposure and bounded liquidity for stablecoins on Solana.

## Overview

Equilibrium-Core is a hub-and-spoke AMM designed for stablecoin liquidity optimization. It implements:

- **Weighted Asset Exposure**: Maintains target weights (45% USDC, 35% USDT, 20% PYUSD) through arbitrage incentives
- **Bounded Liquidity**: Concentrates capital in the 0.99-1.01 price range for maximum efficiency
- **Dynamic Fees**: Adjusts swap fees based on pool imbalance to accelerate rebalancing

Built as a technical exploration of advanced AMM mechanics, Equilibrium-Core demonstrates how modern stablecoin pools can achieve superior capital efficiency while maintaining peg stability.

## Key Features

### Hub-and-Spoke Architecture
- Central USD* token acts as the hub for all swaps
- Isolates risk between partner stablecoins
- Enables efficient multi-hop routing

### Weighted Asset Exposure
```rust
// Target weights for Seed Pool
const USDC_WEIGHT: f64 = 0.45;  // 45%
const USDT_WEIGHT: f64 = 0.35;  // 35%
const PYUSD_WEIGHT: f64 = 0.20; // 20%

// Check if pool needs rebalancing
fn needs_rebalance(&self) -> bool {
    let total_value = self.usdc + self.usdt + self.pyusd;
    
    let usdc_weight = self.usdc as f64 / total_value as f64;
    let usdt_weight = self.usdt as f64 / total_value as f64;
    let pyusd_weight = self.pyusd as f64 / total_value as f64;
    
    (usdc_weight - USDC_WEIGHT).abs() > 0.01 || 
    (usdt_weight - USDT_WEIGHT).abs() > 0.01 || 
    (pyusd_weight - PYUSD_WEIGHT).abs() > 0.01
}
```

### Dynamic Fee Model
Fees automatically adjust based on pool imbalance:
- Base fee: 0.05%
- Maximum fee: 0.5%
- Fee increases proportionally to weight deviation

```rust
fn calculate_fee(&self) -> f64 {
    let base_fee = 0.0005; // 0.05%
    let max_fee = 0.005;   // 0.5%
    
    let total_value = self.usdc + self.usdt + self.pyusd;
    let usdc_weight = self.usdc as f64 / total_value as f64;
    let usdt_weight = self.usdt as f64 / total_value as f64;
    let pyusd_weight = self.pyusd as f64 / total_value as f64;
    
    let deviation = (usdc_weight - USDC_WEIGHT).abs() + 
                   (usdt_weight - USDT_WEIGHT).abs() + 
                   (pyusd_weight - PYUSD_WEIGHT).abs();
    
    (base_fee + deviation).min(max_fee)
}
```

## Technical Architecture

### Programs
- **SeedPool**: Manages the core liquidity pool with weighted exposure
- **USDStar**: Implements the hub token (USD*)
- **Router**: Handles optimal swap routing and fee calculation
- **LiquidityManager**: Handles LP deposits/withdrawals

### Integration Points
- **SPL Token**: For creating test tokens (FAKE_USDC, FAKE_USDT, FAKE_PYUSD)

## Development

### Prerequisites
- Rust 1.68+
- Solana CLI 1.14+
- Anchor 0.27+

### Setup
```bash
# Clone the repository
git clone https://github.com/yourusername/equilibrium-core.git
cd equilibrium-core

# Install dependencies
yarn install

# Build the program
anchor build

# Deploy to devnet
anchor deploy --provider.cluster devnet
```

### Testing
```bash
# Run unit tests
anchor test

# Run integration tests on devnet
yarn test:devnet
```

## Future Improvements

- **Concentrated Liquidity**: Implement tighter bounds (0.995-1.005) for even higher capital efficiency
- **Cross-Chain Integration**: Extend the hub-and-spoke model across multiple chains
- **Risk Management**: Add automatic hedging for partner stablecoins

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgements

Inspired by innovations in modern AMM design, particularly weighted exposure mechanisms and bounded liquidity concepts pioneered by leading stablecoin protocols.
