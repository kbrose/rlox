An implementation of the "Lox" language from [_Crafting Interpreters_](https://craftinginterpreters.com/), being filled out as I follow through the (fabulous) book.

The book walks through two separate implementations of the Lox language. The first is an AST-walking interpreter (using Java in the book), the second is a bytecode interpreter (using C in the book).

Both versions are implemented in Rust here, with the first (complete) implementation living in [ast-walker](./ast-walker) and the second (in progress) implementation living in [bytecode](./bytecode).

To run the interpreter(s):

```bash
cd ast-walker
cargo build --release
cd ../bytecode
cargo build --release
cd ..
cp ast-walker/target/release/loxa ./
cp bytecode/target/release/loxb ./
./loxa lox/test.lox  # AST-walker interpreter
./loxb lox/test.lox  # Bytecode interpreter
```
