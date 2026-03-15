# Lindera CRF

Lindera CRF is a pure Rust implementation of Conditional Random Fields (CRFs), forked from [rucrf](https://github.com/daac-tools/rucrf). It provides a trainer and an estimator for CRFs with support for lattice structures.

## Key Features

- Lattices with variable length edges
- L1, L2, and Elastic Net regularization
- Multi-threaded training
- Zero-copy deserialization with rkyv
- `no_std` support (without `train` feature)

## Contents

- [Architecture](./lindera-crf/architecture.md) -- Internal structure and key components
- [API Reference](./lindera-crf/api_reference.md) -- API documentation

## Changes from rucrf

- **Serialization backend**: Switched from `bincode` to `rkyv` for zero-copy deserialization
- **Elastic Net regularization**: Added `Regularization::ElasticNet` combining L1 and L2 penalties
- **Rust 2024 edition**: Updated to Rust 2024 edition
- **Dependency updates**: Updated `argmin`, `argmin-math`, `hashbrown`, etc.
