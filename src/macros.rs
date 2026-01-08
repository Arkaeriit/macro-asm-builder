use crate::Assembler;
use crate::tree::AsmNode::*;
use crate::tree::*;

/// Macros are identified by their name and the number of argument they take.
/// The macro ID is used to index their content (a string that will be
/// processed) in a hash map.
#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub struct MacroID {
    name: String,
    number_of_arguments: usize,
}

impl MacroID {
    /// Generate a new empty macro. Its content will be filled with new lines
    /// of source code.
    fn new(name: &str, number_of_arguments: usize) -> Self {
        MacroID {name: name.to_string(), number_of_arguments}
    }
}

/// Traverse the source in search of `@macro` lines. At those line a new macro
/// is defined and all lines until a `@end` line will be put in the macro.
/// Then, the resulting macro is registered in the assembler's state.
/// All used up lines are replaced with Empty nodes.
pub fn register_macros(asm: &mut Assembler) -> bool {
    let mut registered_macro = false;
    let mut in_macro = false;
    let mut new_macro_id = MacroID::new("", 0);
    let mut new_macro_content = "".to_string();

    let mut register_macros_closure = | node: &AsmNode | -> Option<AsmNode> {
        match node {
            Source{code, meta} => {
                match code[0].as_str() {
                    "@macro" => {
                        if in_macro {
                            Some(Error{msg: "Error, macro definitions can't be nested.".to_string(), meta: meta.clone()})
                        } else if code.len() == 3 {
                            in_macro = true;
                            new_macro_id.name = code[1].clone();
                            let number_of_arguments: Result<usize, _> = code[2].parse();
                            match number_of_arguments {
                                Ok(x) => {
                                    new_macro_id.number_of_arguments = x;
                                    Some(Empty)
                                },
                                Err(_) =>
                                    Some(Error{msg: "Error, macro definitions should have the form `@macro <macro name> <number_of_arguments>`.".to_string(), meta: meta.clone()})
                            }
                        } else {
                            Some(Error{msg: "Error, macro definitions should have the form `@macro <macro name> <number_of_arguments>`.".to_string(), meta: meta.clone()})
                        }
                    },
                    "@end" => {
                        if in_macro {
                            in_macro = false;
                            asm.macros.insert(new_macro_id.clone(), new_macro_content.clone());
                            new_macro_content = "".to_string();
                            registered_macro = true;
                            Some(Empty)
                        } else {
                            Some(Error{msg: "Error, @end should only be used to end macro definitions.".to_string(), meta: meta.clone()})
                        }
                    },
                    _ => {
                        if in_macro {
                            new_macro_content.push_str(&meta.raw);
                            new_macro_content.push('\n');
                            Some(Empty)
                        } else {
                            None
                        }
                    },
                }
            },
            _ => None,
        }
    };

    asm.root.traverse_tree(&mut register_macros_closure);
    if in_macro {
        asm.root.error_on_top(format!("Error, macro {} is not closed with `@end` directive", new_macro_id.name));
    }
    registered_macro
}

/// Perform all the replacement suitable for macro expansion.
fn substitution_for_expansion(expanded_text: &str, pattern_from: &str, pattern_to: &str) -> String {
    let replacement_slashes_escaped = pattern_to.replace("\\", "\\\\");
    let replacement_all_escaped = replacement_slashes_escaped.replace("\"", "\\\"");
    let replacement_quoted = format!("\"{}\"", replacement_all_escaped);
    let replacement_mono_line = replacement_quoted.replace("\n", "\\n");
    expanded_text.replace(pattern_from, &replacement_mono_line)
}

/// Search all the sources lines of the code for macro to be expanded. In those
/// cases, the content of the macro is fetched from the assembler's macro list
/// and the lines are replaced with an Inode containing the expanded macro.
/// expansion_counter is to keep track of how many time macro have been expanded
/// to make unique special expansion for each macro instance.
pub fn expand_macros(asm: &mut Assembler) -> bool {
    let mut expansion_counter = asm.macro_expansion_count;
    let ret = expand_macros_with_explicit_counter(asm, &mut expansion_counter);
    asm.macro_expansion_count = expansion_counter;
    ret
}

/// Same as expand_macro, but the expansion_counter can be given to be
/// incremented in sub-assemblers.
fn expand_macros_with_explicit_counter(asm: &mut Assembler, expansion_counter: &mut usize) -> bool {
    let mut expanded_any_macro = false;

    let mut expand_macros_closure = | node: &AsmNode | -> Option<AsmNode> {
        // All the work is done in this function. The outer closure is needed to
        // access the macro list of the Assembler; but we need to do the
        // expansion recursively, which cannot be done from the closure and we
        // must call expand_macros inside of it.
        match node {
            Source{code, meta} => {
                let number_of_arguments = code.len() - 1;
                let macro_id = MacroID::new(&code[0], number_of_arguments);
                match asm.macros.get(&macro_id) {
                    Some(macro_txt) => {
                        // Formatting name for error reporting
                        let mut macro_name = "macro_".to_string();
                        macro_name.push_str(&code[0]);
                        // Arguments substitution
                        let mut expanded_text = macro_txt.clone();
                        for i in 0..number_of_arguments {
                            let pattern = format!("${}", i+1);
                            expanded_text = substitution_for_expansion(&expanded_text, &pattern, &code[i+1]);
                        }
                        // Unique symbol expansion
                        let unique_symbol = format!("usx_{expansion_counter:x}");
                        expanded_text = substitution_for_expansion(&expanded_text, "$?", &unique_symbol);
                        *expansion_counter += 1;
                        // Result generation
                        let mut expanded_macro = Assembler::from_named_text(&expanded_text, &format!("`macro '{}' expanded from file {} at line {}`", macro_name, meta.source_file, meta.line));
                        expanded_macro.macros = asm.macros.clone();
                        expanded_macro.macros.remove(&macro_id); // Remove the macro name to prevent infinite recursion. Instead an error will be raised when the macro is not found. Furthermore, this can be used to shadow macros or instructions.
                        expand_macros_with_explicit_counter(&mut expanded_macro, expansion_counter);
                        expanded_any_macro = true;
                        Some(expanded_macro.root)
                    },
                    None => None,
                }
            },
            _ => None,
        }
    };


    asm.root.traverse_tree(&mut expand_macros_closure);
    expanded_any_macro
}

/* --------------------------------- Testing -------------------------------- */

#[test]
fn test_register_macros() {
    let mut assembler = Assembler::from_text("@macro my_macro 3\nmacromacro\ntxttxt\n@end");
    let expected_hash_map = std::collections::HashMap::from([
        (MacroID{name: "my_macro".to_string(), number_of_arguments: 3}, "macromacro\ntxttxt\n".to_string()),
    ]);

    register_macros(&mut assembler);
    assert_eq!(expected_hash_map, assembler.macros);
}

#[test]
fn test_expand_macro_no_arg_substitution() {
    let mut assembler = Assembler::from_text("@macro m 0\nx x\n@end\nm\nm\n");
    register_macros(&mut assembler);
    expand_macros(&mut assembler);
    assert_eq!(assembler.root.to_string(), "x x\nx x\n");
}

#[test]
fn test_expand_macro() {
    let mut assembler = Assembler::from_text("@macro m 2\nx $1 $2\n@end\nm a A\nm b B\n");
    register_macros(&mut assembler);
    expand_macros(&mut assembler);
    assert_eq!(assembler.root.to_string(), "x a A\nx b B\n");
}

#[test]
fn test_expand_macro_recursive() {
    let mut assembler = Assembler::from_text("@macro m 2
                                                  $1 $2
                                              @end
                                              @macro in 1
                                                  x $1
                                              @end
                                              m in a
                                              m in b");
    register_macros(&mut assembler);
    expand_macros(&mut assembler);
    assert_eq!(assembler.root.to_string(), "x a\nx b\n");
}

#[test]
fn test_multiple_macro_with_the_same_name() {
    let mut assembler = Assembler::from_text("@macro dub 1
                                                  dub $1 $1
                                              @end
                                              @macro dub 2
                                                  $1 $2 $1 $2
                                              @end
                                              dub a b
                                              dub c");
    register_macros(&mut assembler);
    expand_macros(&mut assembler);
    assert_eq!(assembler.root.to_string(), "a b a b\nc c c c\n");
}

#[test]
fn test_expansion_unique_symbol() {
    let mut assembler = Assembler::from_text("@macro m1 0
                                                  m2
                                                  $?
                                              @end
                                              @macro m2 0
                                                  $?
                                              @end
                                              m1
                                              m2
                                              m1");
    register_macros(&mut assembler);
    expand_macros(&mut assembler);
    assert_eq!(assembler.root.to_string(), "usx_1\nusx_0\nusx_2\nusx_4\nusx_3\n");
}

