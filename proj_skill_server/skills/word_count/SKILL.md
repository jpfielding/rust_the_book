# word_count

Pipes the run's input through `wc -w` and returns the count. Still
synchronous-friendly (fast, single line of output) — useful for
sanity-checking M2 before you build the streaming path.
