# lindera-trainer

CRF-based dictionary training for [Lindera](https://github.com/lindera/lindera).

This crate implements the training pipeline behind the `lindera train` and
`lindera export` commands:

```text
lindera train  →  trained model (.dat)  →  lindera export  →  dictionary source files  →  lindera build
```

It is normally consumed through the `lindera` facade crate with the `train`
feature enabled, which re-exports this crate as `lindera::dictionary::trainer`:

```toml
[dependencies]
lindera = { version = "5.0", features = ["train"] }
```

## License

MIT
