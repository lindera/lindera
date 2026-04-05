<?php declare(strict_types=1);
/*
 * This file is part of sebastian/git-state.
 *
 * (c) Sebastian Bergmann <sebastian@phpunit.de>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */
namespace SebastianBergmann\GitState;

use const DIRECTORY_SEPARATOR;

/**
 * @no-named-arguments Parameter names are not covered by the backward compatibility promise for sebastian/git-state
 *
 * @internal This class is not covered by the backward compatibility promise for sebastian/git-state
 */
final readonly class GitCommandRunner
{
    private ShellCommandRunner $runner;

    public function __construct(?ShellCommandRunner $runner = null)
    {
        if ($runner === null) {
            // @codeCoverageIgnoreStart
            $runner = new ShellCommandRunnerImplementation;
            // @codeCoverageIgnoreEnd
        }

        $this->runner = $runner;
    }

    /**
     * @param non-empty-string $command
     */
    public function run(string $command): false|string
    {
        $command = 'git ' . $command;

        if (DIRECTORY_SEPARATOR === '/') {
            $command = 'LC_ALL=en_US.UTF-8 ' . $command;
        }

        return $this->runner->run($command);
    }
}
