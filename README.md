# Macro-asm-builder

A library to build macro assemblers/linker that generates raw binary files. The development on this library started in the repository [Reflet](https://github.com/Arkaeriit/reflet/tree/f3646a5dbbfabca683dd4f85f6c19cc13f388cae/assembler) until the commit `f3646a5dbbfabca683dd4f85f6c19cc13f388cae`, after which it was moved to its own repository.

## Features 

The library let you add the following features in your macro-assembler:

### Assembly language file

Each line can contain a line with an instruction (`add R3`), a line with no code (just white-space or a comment), an user-defined macro or an assembler directive. The various fields in an instruction or macro should not be separated by commas.

### Comments

Comments start with a semi-colon and end at the end of the line. They are ignored by the assembler.

### Instructions

Instruction start with the mnemonic for the operation and then, if the instruction needs it, an argument that can be a register, a two bit number or a four bit number. Instruction are case-insensitive.

### User-defined macros

Macros are defined in the following way:
```
@macro <macro_name> <number of args>
    <content>
    <content>
@end
```

In the macro's content, arguments are accessed by calling `$<argument_number>`

In assembly, macros are called by doing `<macro_name> <arg1> <arg2> ...`

Note: The first argument is accessed with $1.

### Flat defines

User-defined macros are only expanded if they are the first element in a line. If you want to have a word be a synonym for any placed words, you can use flat defined as follow:

```
@define two 2
@define r4_two R4 two

set two ; Will be expanded to `set 2`
setr r4_five ; Will be expanded st `setr R4 2`
```

If you want to do some compile-time computation in your flat defines, you can use the `@define-math` directive as follow:

```
@define-math four 3 + 1
@define-math height four * 2

set height ; Will be expanded to `set 8`
```

You can use the value of words defined in `@define-math` in other `@define-math`, but not the ones defined in `@define`. `@define-math` uses [Math-Parse](https://github.com/Arkaeriit/math-parse) under the hood, you can use all it's integer functions.

### Sections

By default, the content in the generated binary file follows the order of the content in the source files. But you might want to have more flexibility in where to place code or data. You can do so with the `@section <section name>` directive to declare the position of a section. Then, you can put content at that section by placing it between the directives `@in-section <section name>` and `@end-section`.

Nesting section is not recommended and might result in unexpected behaviors. 

### Assembler directives

Beyond the directives to declare macros and sections, the assembler offers other directives.

* `@label <name>` defines a position in the code.
* `@labref <name>` write in the code the address of the label with the given name.
* `@align <number>` align the next instruction to the number given in bytes.
* `@pad-until <address>` Add padding until the given address is reached.
* `@constant <number>` put the value of the number in the machine code.
* `@rawbytes <byte 1> <byte 2> ... <byte N>` writes the given bytes (in hexadecimal) in the machine code.
* `@string "..."` writes the strings in the quotes following the directive in the machine code.
* `@import <path>` include in the assembly the content of the given file. The path is relative to the path of the file where the import directive is.

As alternatives to `@string`, there is also `@string-nl`, `@string-0`, and `@string-nl-0` which add respectively a new line, a null byte, and a new line followed by a null byte after the string.

## Usage

Usage of the macro-asm-builder is done in the following way:

```rust
// Initializing the assembler with an input assembly text file
let mut asm = macro_asm_builder::Assembler::from_file("file path");

// Indicating the function that transform raw instrinctions into machine code
asm.micro_assembly = &my_micro_assembly_function;

// Optionaly adding a function to handle custom macros
asm.implementation_macro &my_macro_function

// Generating machine code
match asm.assemble() {
    Err(txt) => {
        // Print error message
        eprintln!("{txt}");
    },
    Ok(machine_code) => {
        // Here, the machine code is ready
    },
}
```

Internally, the lines of assembly language are handled as list of string, each string being a word. As such, the two functions you must plug in the assembler are:
* `micro_assembly`: Takes a `&Vec<String>` as argument, returns `Ok(Vec<u8>)` which contains bytes of machine code if the input is correct or `Err(String)` which contains an error messages otherwise.
* `implementation_macro`: Takes a `&Vec<String>` as argument. If the input should be expanded as a macro, it should return `Ok(Some(String))` with expended assembly language. If no macro expansion is needed, it should return `Ok(None)`. To return an error message, it should give `Err(String)`.

## Examples

A very simple example of use is in this repository, under `tests/macro_asm_builder_test.rs`. A more complex assembler built with it is the [Reflet assembler](https://github.com/Arkaeriit/reflet/tree/master/assembler).

