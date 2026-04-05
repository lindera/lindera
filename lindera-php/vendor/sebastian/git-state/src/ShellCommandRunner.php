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

/**
 * @no-named-arguments Parameter names are not covered by the backward compatibility promise for sebastian/git-state
 *
 * @internal This class is not covered by the backward compatibility promise for sebastian/git-state
 */
interface ShellCommandRunner
{
    /**
     * @param non-empty-string $command
     */
    public function run(string $command): false|string;
}
