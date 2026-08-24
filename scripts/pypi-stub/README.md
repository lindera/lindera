# lindera-python is now lindera

As of v6.0.0, Lindera's Python binding is published on PyPI as
[`lindera`](https://pypi.org/project/lindera/). The `lindera-python`
distribution is no longer updated.

Installing this package pulls in `lindera` automatically, but please switch
your dependency declarations to the new name:

```bash
pip uninstall lindera-python
pip install lindera
```

Your code does not change — the import name has always been `lindera`:

```python
import lindera
```
