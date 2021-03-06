//! Adapts a raw elm token stream to be parsable by a conventional parser.
//!
//! With Newline tokens, places closing delimiters in `case` expressions so
//! an LR parser can properly parse the lexed stream.
use std::fmt::{self,Debug};
use std::iter::Peekable;

use ::tokens::ElmToken;

pub type Wrap<X,W> = (W,X);

#[derive(PartialEq, Copy, Clone)]
enum IndentEntry {
    Let(u32),
    Case(u32),
    // The delimiters are `let .. in`, `( .. )`, `[ .. ]`, `{ .. }`,
    // `if .. then .. else`.
    // Keeping track of them helps telling where to insert the `endcase`
    // tokens.
    Delimiter,
    // Something that should never be put in the stack, to indicate
    // we want to reach the bottom of the stack
    Bottom,
}

impl fmt::Debug for IndentEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IndentEntry::Let(indent) => write!(f, "l{}", indent),
            IndentEntry::Case(indent) => write!(f, "c{}", indent),
            IndentEntry::Delimiter => write!(f, "del"),
            IndentEntry::Bottom => write!(f, "_|_"),
        }
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
enum IndentTrigger {Let, Of}

/// An iterator adaptator that turns Newline tokens into meaningfull
/// indentation for the parser to consume.
///
/// The `FilterIndent` keeps as internal state the indent stack. The
/// indent stack keeps track of meaningfull indentations for when
/// they need to be aligned, and for inserting "endcase" tokens when
/// necessary (nested `case` expressions). The `indent_stack` not only
/// keeps track of what "meaningfull" indentation lines to look for, but
/// also of enclosing delimiters such as `()`, `[]` or `{}`. This is so
/// we do not insert `endcase` tokens at the wrong place.
pub struct FilterIndent<W:Copy+Debug,I:Iterator<Item=Wrap<ElmToken,W>>> {
    input: Peekable<I>,
    // Holds information about the levels of indentation we are in, and
    // if there is "enclosing" expressions that renders `Of` ending moot.
    indent_stack: Vec<IndentEntry>,
    // If a keyword into an expression needing indentation is found, put
    // it there so at the next newline we know what to do
    indent_trigger: Option<IndentTrigger>,
    // A buffer to hold multiple tokens we want to emit later.
    buffer_stack: Vec<Wrap<ElmToken,W>>,
    let_alignement: Option<u32>,
    last_loc: W,
}

impl<W:Copy+Debug,I:Iterator<Item=Wrap<ElmToken,W>>> FilterIndent<W,I> {
    // Pop `indent_stack`, closing off unfinished Case expressions by
    // pushing `Endcase` in `buffer_stack`. `indent_stack` is poped until
    // given `entry` is found, the corresponding item is poped from the
    // stack.
    // We only push `Endcase` when there is two succeeding `Of` sources.
    // This follows the grammatical rules of only closing `case` expressions
    // with an `endcase` if the `case` are nested or nested within open
    // expressions.
    //
    // ## Notes
    // If the entry is not in the stack, the whole stack will be popped
    fn pop_indents_to(&mut self, entry: IndentEntry) {
        let mut last_indenter_is_case = false;
        loop { match self.indent_stack.pop() {
            None => return,
            Some(IndentEntry::Case(indent_level)) if last_indenter_is_case => {
                self.buffer_stack.push((self.last_loc, ElmToken::Endcase));
                if IndentEntry::Case(indent_level) == entry { return }
            },
            Some(IndentEntry::Case(indent_level)) /* otherwise */ => {
                last_indenter_is_case = true;
                if IndentEntry::Case(indent_level) == entry { return }
            },
            Some(indenter) if indenter == entry => return,
            Some(_) /* otherwise */ => {
                last_indenter_is_case = false;
            },
        } }
    }

    // Finds the corresponding indent in `indent_stack`.
    // pop `indent_stack` and inserts token in `buffer_stack` accordingly
    //
    // Ideally this should be inlined in the only place where calling this
    // function is relevent (in source code)
    fn locate_indent(&mut self, indent_level: u32) {
        // This function works in two steps:
        // 1. detects if an entry in the `indent_stack` has the corresponding
        //    `indent_level`.
        // 2. If not, returns.
        //    otherwise pops `indent_stack` up to the found `indent_level`
        use self::IndentEntry::{Case,Let};
        if self.indent_stack.is_empty() { return }
        let stack_last = self.indent_stack.len() - 1;
        match {
            let mut matching_index = stack_last;
            loop {
                let current_indenter
                    = self.indent_stack[matching_index];
                match current_indenter {
                    Case(indent) if indent == indent_level => {
                        let loc = self.last_loc;
                        self.buffer_stack.push((loc, ElmToken::CaseIndent));
                        break Some(matching_index)
                    },
                    Let(indent) if indent == indent_level => {
                        let loc = self.last_loc;
                        self.buffer_stack.push((loc, ElmToken::LetIndent));
                        break Some(matching_index)
                    },
                    _ => {},
                }
                if matching_index == 0 { break None }
                matching_index -= 1;
            }
        } { // 2.
            None => return,
            Some(indenter_location) => {
                let pop_count = stack_last - indenter_location;
                let mut last_indenter_is_case = false;
                // Pop values over the indentation we found (leaving
                // the one we found)
                // pop_count is lower than the size of the indent_stack,
                // therefore no panic in the `unwrap`
                for _ in 0..pop_count {
                    match self.indent_stack.pop().unwrap() {
                        Case(_) if last_indenter_is_case => {
                            let loc = self.last_loc;
                            self.buffer_stack.push((loc,ElmToken::Endcase));
                        },
                        Case(_) => last_indenter_is_case = true,
                        _ => last_indenter_is_case = false,
                    }
                }
                // Cleanup if we popped a case when we are aligned to a case
                if last_indenter_is_case
                   && self.indent_stack.last() == Some(&Case(indent_level))
                {
                    let loc = self.last_loc;
                    self.buffer_stack.push((loc,ElmToken::Endcase))
                }
            },
        }
    }
}

impl<W:Copy+Debug,I:Iterator<Item=Wrap<ElmToken,W>>> fmt::Debug for FilterIndent<W,I> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let &FilterIndent{
            ref indent_stack,
            ref indent_trigger,
            ref buffer_stack, ..} = self;
        write!(f, "FilterIndent {{input:Iterator<Item=ElmToken>, \
                indent_stack: {:?},\
                indent_trigger: {:?},\
                buffer_stack: {:?} }}", indent_stack, indent_trigger,
                buffer_stack)
    }
}

impl<W:Copy+Debug,I:Iterator<Item=Wrap<ElmToken,W>>> Iterator for FilterIndent<W,I> {
    type Item=Wrap<ElmToken,W>;

    fn next(&mut self) -> Option<Wrap<ElmToken,W>> {
        use self::ElmToken::*;
        use self::IndentTrigger as IT;
        use self::IndentEntry as IE;

        if let Some(token) = self.buffer_stack.pop() { return Some(token) }
        let let_alignement = self.let_alignement.take();

        match self.input.next() {
            Some((loc,token)) => {
            self.last_loc = loc;
            match token {
                LParens | LBrace | LBracket | If => {
                    self.indent_stack.push(IE::Delimiter);
                    return Some((loc,token));
                },
                Let => {
                    self.indent_trigger = Some(IT::Let);
                    self.indent_stack.push(IE::Delimiter);
                    match self.input.peek() {
                        Some(&(_,Newline(_))) | None => {},
                        Some(_) => if let Some(alignement) = let_alignement {
                            self.indent_stack.push(IE::Let(alignement));
                            self.indent_trigger = None;
                        },
                    };
                    return Some((loc,Let));
                },
                Of => {
                    self.indent_trigger = Some(IT::Of);
                    return Some((loc,Of));
                },
                RParens | RBrace | RBracket | Else => {
                    self.buffer_stack.push((loc,token));
                    self.pop_indents_to(IE::Delimiter);
                },
                In => {
                    self.indent_trigger = None;
                    self.buffer_stack.push((loc,In));
                    self.pop_indents_to(IE::Delimiter);
                },
                Newline(column) if column != 0 => {
                    if let Some(&(_,Let)) = self.input.peek() {
                        self.let_alignement = Some(column + 4)
                    };
                    match self.indent_trigger {
                        Some(IT::Let) => {
                            self.indent_trigger = None;
                            self.indent_stack.push(IE::Let(column));
                        },
                        Some(IT::Of) => {
                            self.indent_trigger = None;
                            self.indent_stack.push(IE::Case(column));
                        },
                        None => match self.input.peek() {
                            Some(&(_,Operator(_))) => {},
                            _ => self.locate_indent(column),
                        },
                    }
                },
                any_other => return Some((loc,any_other)),
            }
            },
            None if !self.indent_stack.is_empty() =>
                // End of input, need to wrap up
                // Calling pop_indents_to with the instruction of popping the
                // whole stack
                self.pop_indents_to(IE::Bottom),
            None => return None,
        }
        // if the match expression didn't return, we fall through to a
        // recursive call to self. Note that this is definitively equivalent
        // to a while loop.
        self.next()
    }
}

pub trait TokenIterator<W:Copy+Debug>: Iterator<Item=Wrap<ElmToken,W>> {
    fn filter_indent(self, init_loc: W) -> FilterIndent<W,Self> where Self: Sized {
        FilterIndent {
            input: self.peekable(),
            indent_stack: Vec::new(),
            indent_trigger: None,
            buffer_stack: Vec::new(),
            let_alignement: None,
            last_loc: init_loc,
        }
    }
}
impl<W:Copy+Debug,T: Iterator<Item=Wrap<ElmToken,W>>> TokenIterator<W> for T {}
