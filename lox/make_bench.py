from pathlib import Path
import random

random.seed(0x2026_07_20)

with open(Path(__file__).parent / 'bench.lox', 'w') as f:
    num_parens = 0
    f.write('"Hello, World!"')
    options = [',', '!=', '==', '>', '>=', '<', '<=', '-', '+', '*']
    for i in range(1_000_000):
        binary_op = random.choice(options)

        prefix = random.choice(['', '!', '-', '('])
        expression = random.choice([123, 456.789, '"abc"', '"defghi"'])
        if prefix == '(':
            num_parens += 1
        f.write(f" {binary_op} {prefix}{expression}")
        if num_parens > 0 and random.random() > 0.25:
            num_parens -= 1
            f.write(')')

    f.write(')' * num_parens)
