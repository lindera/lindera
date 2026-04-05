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
 */
final readonly class State
{
    /**
     * @var non-empty-string
     */
    private string $originUrl;

    /**
     * @var non-empty-string
     */
    private string $branch;

    /**
     * @var non-empty-string
     */
    private string $commit;
    private bool $clean;
    private string $status;

    /**
     * @param non-empty-string $originUrl
     * @param non-empty-string $branch
     * @param non-empty-string $commit
     */
    public function __construct(string $originUrl, string $branch, string $commit, bool $clean, string $status)
    {
        $this->originUrl = $originUrl;
        $this->branch    = $branch;
        $this->commit    = $commit;
        $this->clean     = $clean;
        $this->status    = $status;
    }

    /**
     * @return non-empty-string
     */
    public function originUrl(): string
    {
        return $this->originUrl;
    }

    /**
     * @return non-empty-string
     */
    public function branch(): string
    {
        return $this->branch;
    }

    /**
     * @return non-empty-string
     */
    public function commit(): string
    {
        return $this->commit;
    }

    public function isClean(): bool
    {
        return $this->clean;
    }

    public function status(): string
    {
        return $this->status;
    }
}
