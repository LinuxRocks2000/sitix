/*  Rasta: templating language in Rust, meant for speed and verbosity and JSON-ey-ness.
    Any files can be Rasta - it's determined by the opening flag ("[=]") and closing flag ("[/]").

[=-]
    File content here!
[/]

    Like with Liquid, Rasta uses enclosed commands. They are like so:
[# Rasta Comment ]
    The first character after the opening [ is the control character. It can be "#" (comment), "^" (variable read), "=" (variable set), "!" (template set).
    If there is a dash right before the closing ], WITHOUT a space, and a [/] like
[^ variable_with_a_default_value -] default value: used if the variable is not defined [/]
    then everything between the opening [ -] and closing [/] will be considered data. The opening flag defaults to content.
    All files *must* have Rasta content, but they can also have other fields after the content field. Nested variables will be
    considered inside each other, so
[=-]
    [=hello-]
        [=world-]
            Foo, Bar, Baz.
        [/]
    [/]
    The string stored in world is: [^hello.world]
[/]

    Variables are scoped, so you can also do
[=-]
    [=hello-]
        [=world-]
            Foo, Bar, Baz.
        [/]
        The string stored in world is: [^world]
    [/]
    The string stored in world is: [^hello.world]
    [^hello]
[/]

    Without the [^hello], "The string stored in world is: [^world]" will never be displayed. This is because most templates render
    only [^content] and don't bother with, say, [^content.hello].

    The default template for each page is just that - "default". This requires a "default.html" in your templates directory.
    You can use different templates with the [=template <template_name>] flag at the head of the file, like
[=template my_template]
[=-]
    Templated into my_template.html instead of default.html!
[/]
*/


use std::rc::Rc;


trait FancyIO {
    fn read_char(&mut self) -> char; // Pop one byte out of the buffer and return it.

    fn is_empty(&mut self) -> bool;

    fn undo(&mut self); // push back the last removed character.

    fn read_until(&mut self, end : char) -> String { // REQUIRED BEHAVIOR: Empty the buffer up to *AND INCLUDING* the next occurrence of any character matching "end".
        // If the next byte in the buffer is a match, it should return empty and not flush it - while being somewhat inconvenient, this is necessary for reliable behavior.
        let mut ret = String::new(); // slow, lazy implementation. needs work!
        while !self.is_empty() {
            let b = self.read_char();
            if b == end {
                break;
            }
            ret.push(b);
        }
        ret
    }

    fn read_until_escape(&mut self, end : char) -> String { // Same behavior as read_until, but \ will escape matches.
        let mut ret = String::new(); // slow, lazy implementation. needs work!
        let mut escape = false;
        while !self.is_empty() {
            let b = self.read_char();
            if !escape && b == end {
                break;
            }
            escape = b == '\\';
            ret.push(b);
        }
        ret
    }

    fn trim(&mut self) { // purge whitespace until it reaches a non-whitespace character
        if self.is_empty() {
            return; // short circuit
        }
        let mut b : char = self.read_char();
        while b.is_whitespace() && !self.is_empty() {
            b = self.read_char();
        }
        self.undo();
    }

    fn dump(&mut self) -> String { // THIS IS THE DEFAULT WAY OF DUMPING, BUT IT IS BAD! INSTEAD, OVERRIDE THIS WITH A CLEANER IMPLEMENTATION!
        let mut ret = String::new();
        while !self.is_empty() {
            ret.push(self.read_char());
        }
        ret
    }
}


struct FancyFile {
    data : Vec<char>,
    last : char
}

struct FancyString {
    data : Vec<char>,
    last : char
}


use std::io::Read;


impl FancyFile {
    fn new(file : &mut std::fs::File) -> Result<Self, Box<dyn std::error::Error + 'static>> {
        let mut s = String::new();
        file.read_to_string(&mut s)?;
        let mut v : Vec<char> = s.chars().collect();
        v.reverse();
        Ok(Self {
            data : v,
            last : '\0'
        })
    }
}

impl FancyString {
    fn new(data : &str) -> Self {
        let mut v : Vec<char> = data.chars().collect();
        v.reverse();
        Self {
            data : v,
            last : '\0'
        }
    }
}


impl FancyIO for FancyFile {
    fn read_char(&mut self) -> char {
        self.last = self.data.pop().unwrap();
        self.last
    }

    fn is_empty(&mut self) -> bool {
        self.data.len() == 0
    }

    fn undo(&mut self) {
        self.data.push(self.last);
    }
}


impl FancyIO for FancyString {
    fn read_char(&mut self) -> char {
        self.last = self.data.pop().unwrap();
        self.last
    }

    fn is_empty(&mut self) -> bool {
        self.data.len() == 0
    }

    fn undo(&mut self) {
        self.data.push(self.last);
    }
}


#[derive(Debug)]
pub enum LexerToken {
    PlainText (String), // regular plaintext
    SimpleTag (char, String), // tag without extended content
    ExtTag (char, String), // tag with extended content. I love rust enums.
    ClosingTag // just the /
}


pub fn lexer(f : &mut std::fs::File) -> Result<Vec<LexerToken>, Box<dyn std::error::Error + 'static>> { // TODO: make this not public
    let mut buffer = FancyFile::new(f)?;
    let mut ret = vec![];
    while !buffer.is_empty() {
        let plaintext = buffer.read_until_escape('[');
        if plaintext.len() > 0 {
            ret.push(LexerToken::PlainText(plaintext));
        }
        if buffer.is_empty() {
            break;
        }
        buffer.trim();
        let control = buffer.read_char();
        let content = buffer.read_until_escape(']').trim().to_string();
        if control == '/' {
            ret.push(LexerToken::ClosingTag);
        }
        else if control != '#' { // don't parse comments
            if content.len() > 0 && content.chars().last().unwrap() == '-' {
                let mut cars = content.chars();
                cars.next_back();
                ret.push(LexerToken::ExtTag (control, cars.as_str().trim().to_string()));
            }
            else {
                ret.push(LexerToken::SimpleTag (control, content));
            }
        }
    }
    Ok(ret)
}


#[derive(Debug, Clone)]
enum Operation {
    Assignment (String, String), // write a variable
    Label (String, Option<String>), // read a variable, with optional default value (if it don't exist)
    Text (String), // this is just plaintext, to be immediately rendered
}


impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Assignment (name, value) => {
                write!(f, "={} ({})", if name.len() > 0 { name.clone() } else { "content".to_string() }, value)
            },
            Operation::Text (text) => {
                write!(f, "\x1b[33m{:?}\x1b[0m", text)
            },
            Operation::Label (thing, Some(default)) => {
                write!(f, "Label \x1b[32m{}\x1b[0m, inline default \x1b[34m{}\x1b[0m", thing, default)
            },
            Operation::Label (thing, None) => {
                write!(f, "Label \x1b[32m{}\x1b[0m, no inline default", thing)
            }
        }
    }
}


#[derive(Debug)]
pub struct TreeNode {
    operation : Operation,
    children : Vec<TreeNode>
}


pub struct Scope {
    pub name : String,
    parent : Option<Rc<RefCell<Scope>>>,
    children : Vec<Rc<RefCell<Scope>>>,
    content : String
}


impl Scope {
    pub fn top() -> Self {
        Self {
            name : "page".to_string(),
            parent : None,
            content : String::new(),
            children : vec![]
        }
    }

    pub fn print_debug_info(&self) {
        let mut kids : Vec<String> = vec![];
        for child in &self.children {
            kids.push(child.borrow().name.clone());
        }
        println!("Scope with name {} and children {:?}", self.name, kids);
    }

    pub fn draw_tree(&self, mut level : usize) {
        println!("{}- {}", "  ".repeat(level), self.name);
        level += 1;
        println!("{}{}", "  ".repeat(level), self.content);
        for child in &self.children {
            child.borrow().draw_tree(level);
        }
    }

    pub fn wrap(self) -> Rc<RefCell<Scope>> {
        Rc::new(RefCell::new(self))
    }

    pub fn chitlin_w(parent : Rc<RefCell<Scope>>, name : String) -> Rc<RefCell<Scope>> { // make a wrapped scope the parent of a new scope
        let child = Scope {
            name : name,
            parent : Some(parent.clone()),
            content : String::new(),
            children : vec![]
        }.wrap();
        parent.borrow_mut().children.push(child.clone());
        child
    }

    fn get_child(&self, name : &str) -> Option<Rc<RefCell<Scope>>> {
        for child in &self.children {
            if child.borrow().name == name {
                return Some(child.clone());
            }
        }
        None
    }

    fn _get(&self, rid : Vec<&str>, ind : usize) -> Option<String> { // rid will be a vector like ["content", "test", "urmom"].
        // If ind < rid.len - 1, find the child scope referred to by rid[ind] and call that scope's _get, incrementing ind and passing rid without change. 
        // If ind == rid.len - 1, intelligently return whatever content is referred to by that child scope.
        let child_scope = match self.get_child(rid[ind]) {
            Some(scope) => scope,
            None => {
                return None;
            }
        };
        if ind < rid.len() - 1 {
            child_scope.borrow()._get(rid, ind + 1)
        }
        else {
            Some(child_scope.borrow().content.clone())
        }
    }

    fn walk_up(&self, target : &str) -> Option<Rc<RefCell<Scope>>> {
        let mut cursor = self.parent.clone();
        while cursor.is_some() {
            println!("tuba");
            match cursor.clone().unwrap().borrow().get_child(target) {
                Some(_) => return cursor,
                _ => {
                    cursor = cursor.unwrap().borrow().parent.clone();
                }
            }
        }
        println!("Could not find scope referred to by {:?}.", target);
        None
    }

    pub fn get(&self, name : String) -> Option<String> {
        let rid = name.split(".").collect::<Vec<&str>>();
        if self.get_child(rid[0]).is_some() {
            self._get(rid, 0)
        }
        else if self.name == rid[0] { // we looked into the face of the enemy...
            // ...and saw only ourselves staring back at us.
            if rid.len() == 1 {
                Some(self.content.clone())
            }
            else {
                self._get(rid, 1)
            }
        }
        else {
            println!("WALKING ON THE SUN");
            match self.walk_up(rid[0]) {
                Some(scope) => scope.borrow()._get(rid, 0),
                None => None
            }
        }
    }
}


use core::slice::Iter;
use core::iter::Peekable;
use std::cell::RefCell;

impl TreeNode {
    pub fn parse(path : std::path::PathBuf) -> Result<TreeNode, Box<dyn std::error::Error + 'static>> { // this is purely a convenience function. It just calls Congeal.
        let mut file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => {panic!("PANIIIICCCC")}
        };
        let tokens = lexer(&mut file)?;
        let mut tokens = tokens.iter().peekable();
        return Ok(TreeNode::congeal(&mut tokens));
    }

    pub fn is_plaintext(&self) -> bool {
        match self.operation {
            Operation::Text (_) => true,
            _ => false
        }
    }

    pub fn plaintext(&self) -> String {
        match &self.operation {
            Operation::Text (text) => text.to_string(),
            _ => panic!("PANICCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC")
        }
    }

    pub fn congeal(items : &mut Peekable<Iter<'_, LexerToken>>) -> TreeNode {
        let me = match items.next() {
            Some(thing) => thing,
            None => {
                return TreeNode {
                    operation : Operation::Text(String::new()),
                    children : vec![]
                }
            }
        };
        match me {
            LexerToken::PlainText (t) => {
                TreeNode::new_from_op(Operation::Text(t.clone()))
            },
            LexerToken::ExtTag (control, data) => {
                let mut childrets = vec![];
                loop {
                    if
                        match items.peek() {
                            Some(LexerToken::ClosingTag) => true,
                            _ => false
                        }
                    {
                        items.next(); // consume, but ignore, the closing tag
                        break; // break the childret loop
                    }
                    childrets.push(TreeNode::congeal(items));
                }
                TreeNode {
                    operation : TreeNode::make_op(*control, data),
                    children : childrets
                }
            },
            LexerToken::SimpleTag (control, data) => {
                TreeNode {
                    operation : TreeNode::make_op(*control, data),
                    children : vec![]
                }
            },
            _ => {
                panic!("THIS IS SO STUPID");
            }
        }
    }

    fn make_op(control : char, data : &str) -> Operation {
        let mut data = FancyString::new(data);
        match control {
            '=' => {
                Operation::Assignment(data.read_until(' ').trim().to_string(), data.dump().trim().to_string())
            },
            '^' => {
                let name = data.read_until(' ').trim().to_string();
                let dump = data.dump().trim().to_string();
                Operation::Label(name, if dump.len() > 0 { Some(dump) } else { None })
            },
            _ => {
                panic!("TODO: proper error handling");
            }
        }
    }

    fn new_from_op(operation : Operation) -> Self {
        Self {
            operation,
            children : vec![]
        }
    }

    pub fn print(&self) {
        self.print_internal(0);
    }

    fn print_internal(&self, tab_level : usize) {
        println!("{}{}", "  ".repeat(tab_level), self.operation);
        for child in &self.children {
            child.print_internal(tab_level + 1);
        }
    }

    pub fn render(&self, scope : Rc<RefCell<Scope>>) -> String {
        let mut ret = String::new();
        for child in &self.children {
            match child.operation.clone() {
                Operation::Assignment (name, value) => {
                    let child_scope = Scope::chitlin_w(scope.clone(), name);
                    child_scope.borrow_mut().content = if value.trim() == "" { child.render(child_scope.clone()).to_string() } else { value };
                },
                Operation::Text (text) => {
                    /*let mut pruned = text.trim();
                    if pruned == "" {
                        pruned = " ";
                    }*/
                    ret += &text;
                },
                Operation::Label (variable, default) => {
                    let dat : Option<String> = scope.borrow().get(variable);
                    ret += match dat {
                        Some(data) => data,
                        None => {
                            match default {
                                Some(data) => data,
                                None => {
                                    child.render(scope.clone())
                                }
                            }
                        }
                    }.trim()
                }
            }
        }
        scope.borrow_mut().content = ret.clone();
        ret
    }
}