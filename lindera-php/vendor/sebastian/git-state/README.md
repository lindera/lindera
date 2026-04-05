[![Latest Stable Version](https://poser.pugx.org/sebastian/git-state/v)](https://packagist.org/packages/sebastian/git-state)
[![CI Status](https://github.com/sebastianbergmann/git-state/workflows/CI/badge.svg)](https://github.com/sebastianbergmann/git-state/actions)
[![codecov](https://codecov.io/gh/sebastianbergmann/git-state/branch/main/graph/badge.svg)](https://codecov.io/gh/sebastianbergmann/git-state)

# sebastian/git-state

Library for describing the state of a Git checkout.

## Installation

You can add this library as a local, per-project dependency to your project using [Composer](https://getcomposer.org/):

```
composer require sebastian/git-state
```

If you only need this library during development, for instance to run your project's test suite, then you should add it as a development-time dependency:

```
composer require --dev sebastian/git-state
```

## Usage

#### `test.php`
```php
use SebastianBergmann\GitState\Builder;

$builder = new Builder;
$state   = $builder->build();

if ($state === false) {
    // Not a Git repository or no origin remote configured
    exit(1);
}

print $state->originUrl() . PHP_EOL;
print $state->branch() . PHP_EOL;
print $state->commit() . PHP_EOL;

if ($state->isClean()) {
    print 'Working directory is clean' . PHP_EOL;
} else {
    print $state->status() . PHP_EOL;
}
```

```
github.com:sebastianbergmann/git-state.git
main
ab00820c3757dbd30a8caa185aa4515b98713238
M README.md
?? test.php
```
