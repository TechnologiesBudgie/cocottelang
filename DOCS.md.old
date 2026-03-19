# Cocotte Language Documentation

Welcome! Cocotte is a programming language that reads almost like plain English.
If you can write a shopping list, you can learn Cocotte.

---

## Table of Contents

1. [Getting Started](#1-getting-started)
2. [Variables](#2-variables)
3. [Numbers and Math](#3-numbers-and-math)
4. [Text (Strings)](#4-text-strings)
5. [True and False (Booleans)](#5-true-and-false-booleans)
6. [Making Decisions (if/elif/else)](#6-making-decisions)
7. [Repeating Things (Loops)](#7-repeating-things-loops)
8. [Functions](#8-functions)
9. [Lists](#9-lists)
10. [Maps (Dictionaries)](#10-maps-dictionaries)
11. [Classes and Objects](#11-classes-and-objects)
12. [Error Handling](#12-error-handling)
13. [File I/O](#13-file-io)
14. [Modules](#14-modules)
15. [Libraries](#15-libraries)
16. [GUI with Charlotte](#16-gui-with-charlotte)
17. [The Charlotfile (Task Runner)](#17-the-charlotfile-task-runner)
18. [Built-in Functions Reference](#18-built-in-functions-reference)
19. [String Methods Reference](#19-string-methods-reference)
20. [List Methods Reference](#20-list-methods-reference)
21. [Map Methods Reference](#21-map-methods-reference)
22. [CLI Commands Reference](#22-cli-commands-reference)
23. [Complete Example Programs](#23-complete-example-programs)

---

## 1. Getting Started

### Install

Put the `cocotte` binary somewhere in your PATH (e.g. `/usr/local/bin/`).

### Create your first project

```
cocotte init MyProject
cd MyProject
cocotte run
```

That is it. You will see `Hello, World!` printed.

### Run any file

```
cocotte run src/main.cot
```

### Project structure

```
MyProject/
  Millet.toml       # project settings
  Charlotfile        # task runner (like Makefile, but readable)
  src/
    main.cot         # your code goes here
  libraries/         # your local .cotlib files
  modules/           # downloaded modules
  tests/             # test files (*_test.cot)
  dist/              # compiled binaries (created by cocotte build)
```

---

## 2. Variables

A variable is a box that holds a value. You create one with `var`.

```cocotte
var name = "Alice"
var age = 9
var tall = true
var score = 0
```

You can change a variable's value later:

```cocotte
var score = 0
score = score + 10
score = score + 5
print score        # prints 15
```

Variables can hold any type of value. You can even change the type:

```cocotte
var thing = 42
thing = "now I am text"
thing = true
```

### nil — the empty value

`nil` means "nothing" or "no value":

```cocotte
var result = nil
print result       # prints nil
```

---

## 3. Numbers and Math

```cocotte
var a = 10
var b = 3

print a + b        # 13  (add)
print a - b        # 7   (subtract)
print a * b        # 30  (multiply)
print a % b        # 1   (remainder)

# Division uses special syntax to be extra clear:
print divide a by b        # 3.3333...

# Integer division (no decimals):
print floor(divide a by b) # 3
```

### Math functions

```cocotte
print abs(-5)          # 5      (remove minus sign)
print sqrt(16)         # 4      (square root)
print pow(2, 8)        # 256    (power: 2 to the 8th)
print floor(3.9)       # 3      (round down)
print ceil(3.1)        # 4      (round up)
print round(3.5)       # 4      (round to nearest)
print max(10, 20)      # 20     (bigger one)
print min(10, 20)      # 10     (smaller one)
print clamp(150, 0, 100)   # 100 (keep in range)
print sign(-42)        # -1     (-1, 0, or 1)
```

### Formatting numbers

```cocotte
print format_number(3.14159, 2)   # "3.14"
print format_number(1000.0, 0)    # "1000"
```

### Converting between numbers and text

```cocotte
var n = to_number("42")     # text "42" becomes number 42
var s = to_string(3.14)     # number becomes text "3.14"
print n + 1                 # 43
```

### The math module

For more math functions, load the math module:

```cocotte
module add "math"
print math.PI              # 3.14159...
print math.sin(0)          # 0
print math.cos(0)          # 1
print math.log(2.718)      # ~1
print math.sqrt(25)        # 5
print math.pow(2, 10)      # 1024
print math.floor(9.9)      # 9
print math.abs(-7)         # 7
```

---

## 4. Text (Strings)

Text is anything between double quotes `"..."`.

```cocotte
var greeting = "Hello, World!"
var name = "Alice"
```

### Joining text

Use `+` to join two pieces of text:

```cocotte
print "Hello, " + name + "!"     # Hello, Alice!
print "Score: " + 42              # Score: 42  (number joins automatically)
```

### String methods

Call methods with a dot:

```cocotte
var s = "  Hello, World!  "

print s.len()                     # 17 (number of characters)
print s.trim()                    # "Hello, World!" (remove spaces at edges)
print s.upper()                   # "  HELLO, WORLD!  "
print s.lower()                   # "  hello, world!  "
print s.contains("World")         # true
print s.starts_with("  Hello")    # true
print s.ends_with("!  ")          # true
print s.replace("World", "Cocotte")  # "  Hello, Cocotte!  "
print s.split(",")                # ["  Hello", " World!  "]

var clean = s.trim()
print clean.get(0)                # "H" (character at position 0)
print clean.slice(7, 12)          # "World" (characters 7 through 11)
print clean.index_of("World")     # 7 (where does "World" start?)
print "ha".repeat(3)              # "hahaha"
print "7".pad_left(4, "0")        # "0007"
print "hi".pad_right(6, ".")      # "hi...."

print "a,b,c".split(",").join(" + ")   # "a + b + c"
print "line1\nline2\nline3".split_lines()  # ["line1", "line2", "line3"]
```

### Multi-line text

Use `\n` for a new line inside a string:

```cocotte
var poem = "Roses are red\nViolets are blue\nCocotte is great\nAnd so are you"
print poem
```

---

## 5. True and False (Booleans)

```cocotte
var sunny = true
var raining = false

print sunny and raining     # false (both must be true)
print sunny or raining      # true  (at least one must be true)
print not raining           # true  (flip it)
```

### Comparisons (always give true or false)

```cocotte
print 5 > 3       # true
print 5 < 3       # false
print 5 == 5      # true  (equal)
print 5 != 3      # true  (not equal)
print 5 >= 5      # true  (greater than or equal)
print 5 <= 4      # false (less than or equal)
```

---

## 6. Making Decisions

### if / elif / else

```cocotte
var score = 75

if score >= 90
    print "Excellent!"
elif score >= 70
    print "Good job!"
elif score >= 50
    print "Keep trying!"
else
    print "Need more practice."
end
```

Every `if` block ends with `end`. The `elif` and `else` parts are optional.

### Short if

```cocotte
var x = 10
if x > 5
    print "big"
end
```

### Checking multiple things at once

```cocotte
var age = 12
var hasTicket = true

if age >= 10 and hasTicket
    print "You may enter!"
end

if age < 5 or not hasTicket
    print "Sorry, you cannot enter."
end
```

---

## 7. Repeating Things (Loops)

### while — repeat as long as something is true

```cocotte
var count = 1
while count <= 5
    print "Count: " + count
    count = count + 1
end
```

### for — repeat for each item in a list

```cocotte
for fruit in ["apple", "banana", "cherry"]
    print "I like " + fruit
end
```

### for with range — repeat N times

`range(start, end)` gives you a list of numbers from `start` up to (not including) `end`:

```cocotte
for i in range(1, 6)
    print i         # prints 1, 2, 3, 4, 5
end
```

### break and continue

```cocotte
# break — stop the loop early
for n in range(1, 100)
    if n > 5
        break
    end
    print n       # prints 1 through 5
end

# continue — skip this turn, go to next
for n in range(1, 11)
    if n % 2 == 0
        continue       # skip even numbers
    end
    print n            # prints 1 3 5 7 9
end
```

---

## 8. Functions

A function is a reusable block of code with a name. You define it once and call it many times.

### Basic function

```cocotte
func greet(name)
    print "Hello, " + name + "!"
end

greet("Alice")
greet("Bob")
```

### Function that returns a value

```cocotte
func add(a, b)
    return a + b
end

var result = add(3, 4)
print result       # 7
```

### Function with no parameters

```cocotte
func say_hello()
    print "Hello!"
end

say_hello()
```

### Functions as values (lambdas)

You can store a function in a variable:

```cocotte
var double = func(x) return x * 2 end
print double(5)      # 10
print double(21)     # 42
```

### Passing functions to other functions (higher-order)

```cocotte
func apply_twice(f, x)
    return f(f(x))
end

var triple = func(x) return x * 3 end
print apply_twice(triple, 2)    # 18  (2*3=6, 6*3=18)
```

### Closures — functions that remember their surroundings

```cocotte
func make_counter()
    var count = 0
    return func()
        count = count + 1
        return count
    end
end

var counter = make_counter()
print counter()    # 1
print counter()    # 2
print counter()    # 3
```

### Recursion — a function that calls itself

```cocotte
func factorial(n)
    if n <= 1
        return 1
    end
    return n * factorial(n - 1)
end

print factorial(5)     # 120
print factorial(10)    # 3628800
```

---

## 9. Lists

A list holds multiple values in order. Lists use square brackets `[...]`.

```cocotte
var fruits = ["apple", "banana", "cherry"]
var numbers = [1, 2, 3, 4, 5]
var mixed = [1, "hello", true, nil]
var empty = []
```

### Getting items

Positions start at 0:

```cocotte
var fruits = ["apple", "banana", "cherry"]
print fruits.get(0)     # "apple"
print fruits.get(1)     # "banana"
print fruits.get(2)     # "cherry"
print fruits.first()    # "apple"
print fruits.last()     # "cherry"
```

### Adding and removing items

```cocotte
var list = [1, 2, 3]
list.push(4)            # add to end → [1, 2, 3, 4]
list.push(5)            # → [1, 2, 3, 4, 5]
var removed = list.pop()  # remove from end → removed = 5
print list              # [1, 2, 3, 4]
print list.len()        # 4
print list.is_empty()   # false
```

### Looping through a list

```cocotte
var scores = [85, 92, 78, 95, 88]
for score in scores
    print "Score: " + score
end
```

### List operations

```cocotte
var nums = [3, 1, 4, 1, 5, 9, 2, 6]

nums.reverse()                   # flip the list
nums.sort()                      # sort smallest to largest
print nums.contains(5)           # true
print nums.index_of(5)           # position of 5 (or -1 if not found)
print nums.slice(1, 4)           # items at positions 1, 2, 3
print nums.join(", ")            # "1, 1, 2, 3, 4, 5, 6, 9"
```

### Functional list operations

```cocotte
var nums = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

# map — transform every item
var doubled = nums.map(func(x) return x * 2 end)
print doubled    # [2, 4, 6, 8, 10, 12, 14, 16, 18, 20]

# filter — keep only items that pass a test
var evens = nums.filter(func(x) return x % 2 == 0 end)
print evens      # [2, 4, 6, 8, 10]

# reduce — combine all items into one value
var sum = nums.reduce(func(total, x) return total + x end, 0)
print sum        # 55

# each — do something with every item (no return value)
nums.each(func(x) print x end)

# find — get first item matching a test
var firstBig = nums.find(func(x) return x > 5 end)
print firstBig   # 6

# count — how many items pass a test
var bigCount = nums.count(func(x) return x > 5 end)
print bigCount   # 5
```

### Building lists

```cocotte
var squares = []
for i in range(1, 6)
    squares.push(i * i)
end
print squares       # [1, 4, 9, 16, 25]

# Or with map:
var squares2 = range(1, 6).map(func(i) return i * i end)
```

---

## 10. Maps (Dictionaries)

A map stores key-value pairs. Like a real dictionary: look up a word (key), get its definition (value). Maps use curly braces `{...}`.

```cocotte
var person = {
    "name": "Alice",
    "age": 9,
    "city": "Paris"
}
```

### Getting values

```cocotte
print person.get("name")    # "Alice"
print person.get("age")     # 9
```

### Setting values

```cocotte
person.set("age", 10)
person.set("hobby", "coding")
print person.get("age")     # 10
```

### Checking if a key exists

```cocotte
print person.has_key("name")     # true
print person.has_key("phone")    # false
```

### Getting all keys or values

```cocotte
var k = person.keys()     # ["name", "age", "city", "hobby"]
var v = person.values()   # ["Alice", 10, "Paris", "coding"]
print k.len()             # 4
```

### Looping through a map

```cocotte
var config = {"host": "localhost", "port": "8080", "debug": "true"}
for key in config.keys()
    print key + " = " + config.get(key)
end
```

### Maps as records / structs

```cocotte
func make_point(x, y)
    return {"x": x, "y": y}
end

func distance(p1, p2)
    var dx = p1.get("x") - p2.get("x")
    var dy = p1.get("y") - p2.get("y")
    return sqrt(dx * dx + dy * dy)
end

var a = make_point(0, 0)
var b = make_point(3, 4)
print distance(a, b)    # 5
```

---

## 11. Classes and Objects

A class is a blueprint for creating objects. An object is an instance of a class — it has its own data and functions.

### Basic class

```cocotte
class Dog
    func init(name, breed)
        self.name = name
        self.breed = breed
        self.tricks = []
    end

    func bark()
        print self.name + " says: Woof!"
    end

    func learn(trick)
        self.tricks.push(trick)
        print self.name + " learned " + trick + "!"
    end

    func show_tricks()
        print self.name + " knows " + self.tricks.len() + " tricks:"
        for trick in self.tricks
            print "  - " + trick
        end
    end
end

var rex = Dog("Rex", "Labrador")
var fifi = Dog("Fifi", "Poodle")

rex.bark()
rex.learn("sit")
rex.learn("shake")
rex.learn("roll over")
rex.show_tricks()

fifi.bark()
```

### self — the object itself

Inside a class, `self` refers to the current object. Use `self.something` to access or set the object's own data.

```cocotte
class Counter
    func init(start)
        self.value = start
    end
    func increment()
        self.value = self.value + 1
    end
    func decrement()
        self.value = self.value - 1
    end
    func reset()
        self.value = 0
    end
    func get()
        return self.value
    end
end

var c = Counter(0)
c.increment()
c.increment()
c.increment()
print c.get()      # 3
c.decrement()
print c.get()      # 2
c.reset()
print c.get()      # 0
```

### Classes calling their own methods

```cocotte
class Rectangle
    func init(width, height)
        self.width = width
        self.height = height
    end
    func area()
        return self.width * self.height
    end
    func perimeter()
        return 2 * (self.width + self.height)
    end
    func is_square()
        return self.width == self.height
    end
    func describe()
        print "Rectangle " + self.width + "x" + self.height
        print "  Area:      " + self.area()
        print "  Perimeter: " + self.perimeter()
        print "  Square:    " + self.is_square()
    end
end

var r = Rectangle(4, 6)
r.describe()
```

### Using classes as data structures

```cocotte
class Stack
    func init()
        self.items = []
    end
    func push(val)
        self.items.push(val)
    end
    func pop()
        return self.items.pop()
    end
    func peek()
        return self.items.last()
    end
    func size()
        return self.items.len()
    end
    func is_empty()
        return self.items.is_empty()
    end
end

var stack = Stack()
stack.push("first")
stack.push("second")
stack.push("third")
print stack.peek()     # "third"
print stack.pop()      # "third"
print stack.size()     # 2
```

---

## 12. Error Handling

Errors happen — a file might not exist, a number might be wrong. Use `try`/`catch` to handle them gracefully.

```cocotte
try
    var result = divide 10 by 0
catch err
    print "Something went wrong: " + err
end
```

### Handling file errors

```cocotte
try
    var content = read_file("missing_file.txt")
    print content
catch err
    print "Could not read file: " + err
end
```

### Catching any error and continuing

```cocotte
var numbers = ["1", "2", "oops", "4", "five"]
var total = 0

for item in numbers
    try
        total = total + to_number(item)
    catch err
        print "Skipping '" + item + "' — not a number"
    end
end

print "Total: " + total    # Total: 7
```

### The error message

The variable after `catch` holds the error message as a string:

```cocotte
try
    # something that fails
    var x = to_number("not a number")
catch message
    print "Error was: " + message
end
```

---

## 13. File I/O

### Reading and writing files

```cocotte
# Write a file (creates it if it does not exist)
write_file("hello.txt", "Hello, World!\n")

# Read a file (returns its content as a string)
var content = read_file("hello.txt")
print content

# Append to a file (adds to the end without erasing)
append_file("hello.txt", "Second line\n")
append_file("hello.txt", "Third line\n")
```

### Checking if files/dirs exist

```cocotte
print file_exists("hello.txt")    # true
print is_file("hello.txt")        # true
print is_dir("hello.txt")         # false
print is_dir("/tmp")              # true
print file_size("hello.txt")      # size in bytes
```

### Working with directories

```cocotte
make_dir("my_folder")
make_dir("my_folder/subfolder")    # creates nested dirs too

var files = list_dir("my_folder")   # list of filenames
for name in files
    print name
end
```

### Copying, renaming, deleting

```cocotte
copy_file("hello.txt", "hello_backup.txt")
rename_file("hello_backup.txt", "backup.txt")
delete_file("backup.txt")
delete_file("my_folder")      # also works on directories
```

### Reading a file line by line

```cocotte
var content = read_file("data.txt")
var lines = content.split_lines()

for line in lines
    if line.trim().is_empty()
        continue
    end
    print "Line: " + line
end
```

### Writing structured data

```cocotte
module add "json"

var data = {
    "name": "Alice",
    "scores": [95, 87, 92],
    "active": true
}

var text = json.stringify(data)
write_file("data.json", text)

var loaded = json.parse(read_file("data.json"))
print loaded.get("name")      # Alice
```

---

## 14. Modules

Modules add extra features to Cocotte. Load them with `module add`.

### Built-in modules

#### math

```cocotte
module add "math"

print math.PI        # 3.14159265...
print math.E         # 2.71828...
print math.TAU       # 6.28318...

print math.sin(0)
print math.cos(0)
print math.tan(0)
print math.asin(1)
print math.acos(1)
print math.atan(1)
print math.log(math.E)     # natural log
print math.log2(8)         # 3
print math.log10(1000)     # 3
print math.exp(1)          # e
print math.sqrt(25)        # 5
print math.pow(2, 16)      # 65536
print math.floor(3.7)      # 3
print math.ceil(3.2)       # 4
print math.round(3.5)      # 4
print math.abs(-9)         # 9
print math.max(3, 7)       # 7
print math.min(3, 7)       # 3
```

#### json

```cocotte
module add "json"

# Turn any value into a JSON string
var obj = {"x": 1, "y": 2, "tags": ["a", "b"]}
var text = json.stringify(obj)
print text          # {"x":1.0,"y":2.0,"tags":["a","b"]}

# Turn a JSON string back into a Cocotte value
var parsed = json.parse(text)
print parsed.get("x")      # 1
```

#### os

```cocotte
module add "os"

print os.platform()        # "linux", "windows", or "macos"
print os.cwd()             # current working directory
var result = os.exec("echo Hello from shell")
print result               # the command output
```

#### charlotte (GUI)

See Section 16.

---

## 15. Libraries

Libraries are `.cotlib` files you write yourself and share between projects.

### Creating a library

Create `libraries/math_utils.cotlib`:

```cocotte
# math_utils.cotlib

func square(n)
    return n * n
end

func cube(n)
    return n * n * n
end

func is_even(n)
    return n % 2 == 0
end

func is_prime(n)
    if n < 2
        return false
    end
    var i = 2
    while i * i <= n
        if n % i == 0
            return false
        end
        i = i + 1
    end
    return true
end
```

### Using a library

```cocotte
library add "libraries/math_utils.cotlib"

print math_utils.square(5)      # 25
print math_utils.cube(3)        # 27
print math_utils.is_prime(17)   # true
print math_utils.is_even(8)     # true
```

### Adding a library to a project

```
cocotte add /path/to/my_library.cotlib
```

This copies the file to your `libraries/` folder and records it in `Millet.toml`.

---

## 16. GUI with Charlotte

Charlotte is Cocotte's GUI module. It uses egui and works on Linux (Wayland and X11), Windows, and macOS.

### Enabling Charlotte

Charlotte requires extra dependencies. In your `Cargo.toml`, uncomment these lines:

```toml
eframe = "0.29"
egui   = "0.29"

[features]
default = []
gui = ["eframe", "egui"]
```

Then build with:

```
cargo build --release --features gui
```

### Your first window

```cocotte
module add "charlotte"

charlotte.window("Hello App", 400, 300, func(ui)
    ui.heading("Hello, World!")
    ui.label("Welcome to Cocotte GUI")
    if ui.button("Click me!")
        print "Button was clicked!"
    end
end)
```

### Persistent state

Use Maps for state that must survive between frames (the `func(ui)` runs 60 times per second):

```cocotte
module add "charlotte"

var state = {"count": 0, "name": ""}

charlotte.window("Counter", 400, 300, func(ui)
    ui.heading("My Counter App")
    ui.label("Count: " + state.get("count"))

    if ui.button("Add 1")
        state.set("count", state.get("count") + 1)
    end

    if ui.button("Reset")
        state.set("count", 0)
    end

    ui.separator()
    var name = ui.input("name_field", "Enter your name...")
    if name != ""
        ui.label("Hello, " + name + "!")
    end
end)
```

### All UI widgets

#### Text display

```cocotte
ui.label("Normal text")
ui.heading("Big heading")
ui.monospace("fixed width font")
ui.colored_label("red", "This is red text")
ui.colored_label("#FF8800", "This is orange")
ui.separator()           # horizontal line
ui.space()               # small gap
ui.add_space(20)         # gap of 20 pixels
```

#### Buttons and links

```cocotte
if ui.button("Click me")
    print "clicked!"
end

if ui.small_button("Small")
    print "small click"
end

if ui.link("Visit website")
    os.exec("xdg-open https://example.com")
end
```

#### Input fields

```cocotte
# Single line — key must be unique, second arg is placeholder text
var name = ui.input("name_key", "Your name here...")
var email = ui.input("email_key", "Email address...")

# Multi-line
var notes = ui.multiline_input("notes_key", "Write notes here...")
```

#### Checkboxes

```cocotte
# key, label, default value
var enabled = ui.checkbox("enable_key", "Enable feature", false)
if enabled
    ui.label("Feature is ON")
end
```

#### Radio buttons

```cocotte
var color = "red"   # store in a map for real persistence

ui.label("Pick a color:")
if ui.radio("color_key", "Red", "red")
    ui.colored_label("red", "You picked red")
end
if ui.radio("color_key", "Blue", "blue")
    ui.colored_label("blue", "You picked blue")
end
if ui.radio("color_key", "Green", "green")
    ui.colored_label("green", "You picked green")
end
```

#### Sliders

```cocotte
# key, label, min, max, default
var volume = ui.slider("vol", "Volume", 0, 100, 50)
ui.label("Volume: " + volume)

var speed = ui.slider("spd", "Speed", 0.1, 5.0, 1.0)
```

#### Progress bar

```cocotte
var progress = 0.7    # 0.0 to 1.0
ui.progress(progress)
ui.label("70% complete")
```

### Layout

```cocotte
# row — put things side by side
ui.row(func()
    ui.label("Left side")
    ui.label("Right side")
    if ui.button("Middle button")
        print "click"
    end
end)

# column — stack things vertically (useful inside a row)
ui.row(func()
    ui.column(func()
        ui.heading("Column 1")
        ui.label("Item A")
        ui.label("Item B")
    end)
    ui.column(func()
        ui.heading("Column 2")
        ui.label("Item C")
        ui.label("Item D")
    end)
end)

# group — draws a box around content
ui.group(func()
    ui.label("Inside the box")
    if ui.button("Box button")
        print "click"
    end
end)

# scroll — scrollable area
ui.scroll(func()
    for i in range(1, 50)
        ui.label("Item " + i)
    end
end)

# collapsible — hide/show section
ui.collapsible("Advanced Settings", func()
    ui.label("These are hidden by default")
    ui.slider("adv", "Advanced value", 0, 100, 50)
end)
```

### Window size info

```cocotte
var w = ui.available_width()
var h = ui.available_height()
ui.label("Window is " + w + " x " + h + " pixels")
```

---

## 17. The Charlotfile (Task Runner)

The Charlotfile defines tasks — sequences of commands you run often. It is like `make` but readable.

### Basic Charlotfile

```
[project]
name = "MyApp"
author = "Alice"

[tasks.run]
cocotte run

[tasks.build]
cocotte build --release

[tasks.test]
cocotte test

[tasks.clean]
cocotte clean
```

### Running tasks

```
cocotte exec run
cocotte exec build
cocotte exec test
cocotte exec list          # see all available tasks
```

### Variables in Charlotfile

```
[project]
name = "MyApp"

[variables]
BUILD_DIR = "dist"
SERVER_PORT = "8080"

[tasks.serve]
cd ${BUILD_DIR} && python3 -m http.server ${SERVER_PORT}
```

### Multi-step tasks

```
[tasks.deploy]
cocotte build --release
cd dist && zip -r ../myapp.zip .
scp myapp.zip user@server:/var/www/
```

### Multi-language projects

```
[tasks.build_all]
cocotte build --release
cd backend && cargo build --release
cd frontend && npm run build

[tasks.dev]
cocotte run
cd backend && cargo run
```

---

## 18. Built-in Functions Reference

These functions are always available without loading any module.

### Output

| Function | What it does |
|---|---|
| `print value` | Print a value to the terminal |

### Input

| Function | What it does |
|---|---|
| `input("prompt")` | Ask the user to type something, return their answer |

Example:
```cocotte
var name = input("What is your name? ")
print "Hello, " + name + "!"
```

### Math

| Function | What it does |
|---|---|
| `abs(n)` | Absolute value (removes minus sign) |
| `sqrt(n)` | Square root |
| `pow(base, exp)` | Power: base to the exp |
| `floor(n)` | Round down |
| `ceil(n)` | Round up |
| `round(n)` | Round to nearest |
| `max(a, b)` | Bigger of the two |
| `min(a, b)` | Smaller of the two |
| `sign(n)` | -1, 0, or 1 depending on sign |
| `clamp(v, lo, hi)` | Keep v between lo and hi |

### Number formatting and conversion

| Function | What it does |
|---|---|
| `format_number(n, d)` | Format n with d decimal places |
| `number_to_int(n)` | Remove decimal part |
| `to_number(s)` | Convert string to number |
| `to_string(v)` | Convert anything to string |
| `to_bool(v)` | Convert anything to bool |

### Type checking

| Function | What it does |
|---|---|
| `type_of(v)` | Return the type name as a string |
| `is_number(v)` | True if v is a number |
| `is_string(v)` | True if v is a string |
| `is_list(v)` | True if v is a list |
| `is_map(v)` | True if v is a map |
| `is_bool(v)` | True if v is a bool |
| `is_nil(v)` | True if v is nil |
| `is_func(v)` | True if v is a function |

### Characters

| Function | What it does |
|---|---|
| `char_code("A")` | Get the numeric code for a character (65 for "A") |
| `code_char(65)` | Get the character for a numeric code ("A" for 65) |

### Lists

| Function | What it does |
|---|---|
| `range(start, end)` | Create list of numbers from start to end-1 |
| `len(list)` | Number of items in a list |
| `push(list, val)` | Add a value to the end |
| `pop(list)` | Remove and return the last value |
| `reverse(list)` | Reverse the list |
| `sort(list)` | Sort the list |
| `contains(list, val)` | True if the list has this value |
| `list_of(a, b, c...)` | Create a list from the given values |

### Maps

| Function | What it does |
|---|---|
| `keys(map)` | List of all keys |
| `values(map)` | List of all values |
| `has_key(map, key)` | True if the map has this key |
| `map_of(k,v, k,v...)` | Create a map from alternating keys and values |

### Strings

| Function | What it does |
|---|---|
| `len(s)` | Number of characters |
| `upper(s)` | Uppercase version |
| `lower(s)` | Lowercase version |
| `trim(s)` | Remove spaces from both ends |
| `split(s, sep)` | Split into a list by a separator |
| `replace(s, from, to)` | Replace all occurrences |
| `contains(s, sub)` | True if s contains sub |
| `starts_with(s, pre)` | True if s starts with pre |
| `ends_with(s, suf)` | True if s ends with suf |
| `str_join(list, sep)` | Join a list into a string with separator |

### File I/O

| Function | What it does |
|---|---|
| `read_file(path)` | Read entire file, return as string |
| `write_file(path, text)` | Write text to file (overwrites) |
| `append_file(path, text)` | Add text to end of file |
| `delete_file(path)` | Delete a file or directory |
| `file_exists(path)` | True if file or dir exists |
| `is_file(path)` | True if path is a file |
| `is_dir(path)` | True if path is a directory |
| `file_size(path)` | Size in bytes |
| `make_dir(path)` | Create directory (and parents) |
| `list_dir(path)` | List of filenames in a directory |
| `copy_file(from, to)` | Copy a file |
| `rename_file(from, to)` | Move/rename a file |

### System

| Function | What it does |
|---|---|
| `exit(code)` | Stop the program with exit code |
| `env_get(name)` | Read an environment variable |
| `sleep(seconds)` | Wait for a number of seconds |
| `random()` | Random number between 0 and 1 |
| `time_now()` | Current time as seconds since 1970 |

### Testing

| Function | What it does |
|---|---|
| `assert(cond, msg)` | Stop with error if cond is false |
| `assert_eq(a, b)` | Stop with error if a != b |

---

## 19. String Methods Reference

Methods are called with a dot: `"hello".upper()`

| Method | What it does |
|---|---|
| `.len()` | Number of characters |
| `.is_empty()` | True if empty string |
| `.upper()` | All uppercase |
| `.lower()` | All lowercase |
| `.trim()` | Remove whitespace from both ends |
| `.trim_left()` | Remove whitespace from left end |
| `.trim_right()` | Remove whitespace from right end |
| `.get(i)` | Character at position i |
| `.slice(from, to)` | Characters from position from to to |
| `.index_of(sub)` | Position of sub (-1 if not found) |
| `.contains(sub)` | True if contains sub |
| `.starts_with(pre)` | True if starts with pre |
| `.ends_with(suf)` | True if ends with suf |
| `.replace(from, to)` | Replace all occurrences of from with to |
| `.replace_first(from, to)` | Replace only the first occurrence |
| `.split(sep)` | Split into list by separator |
| `.split_lines()` | Split into list by newlines |
| `.repeat(n)` | Repeat the string n times |
| `.pad_left(n, char)` | Pad with char on the left to length n |
| `.pad_right(n, char)` | Pad with char on the right to length n |
| `.to_number()` | Convert to number |
| `.to_list()` | Convert to list of characters |

---

## 20. List Methods Reference

| Method | What it does |
|---|---|
| `.len()` | Number of items |
| `.is_empty()` | True if no items |
| `.get(i)` | Item at position i |
| `.first()` | First item |
| `.last()` | Last item |
| `.push(val)` | Add to end |
| `.pop()` | Remove and return last item |
| `.contains(val)` | True if list has this value |
| `.index_of(val)` | Position of val (-1 if not found) |
| `.slice(from, to)` | Sub-list from position from to to |
| `.find(func)` | First item where func returns true |
| `.filter(func)` | New list keeping items where func returns true |
| `.map(func)` | New list with func applied to every item |
| `.reduce(func, init)` | Combine all items with func, starting from init |
| `.each(func)` | Call func on every item |
| `.count(func)` | Count items where func returns true |
| `.sort()` | Sort in place |
| `.reverse()` | Reverse in place |
| `.join(sep)` | Join all items into a string |
| `.extend(other)` | Add all items from other list |
| `.copy()` | Make a copy of the list |
| `.clear()` | Remove all items |

---

## 21. Map Methods Reference

| Method | What it does |
|---|---|
| `.get(key)` | Value for key (nil if missing) |
| `.set(key, val)` | Set key to val |
| `.has_key(key)` | True if key exists |
| `.keys()` | List of all keys |
| `.values()` | List of all values |
| `.len()` | Number of entries |

---

## 22. CLI Commands Reference

```
cocotte init <name>            Create a new project
cocotte run [file]             Run a source file (default: src/main.cot)
cocotte run --bytecode [file]  Run using bytecode VM (faster for some programs)
cocotte run --debug [file]     Run with debug output
cocotte build [file]           Compile to native binary (output: dist/)
cocotte build --release        Optimized build
cocotte build --os linux       Build for specific OS
cocotte add <module>           Add a built-in module
cocotte add <file.cotlib>      Add a local library file
cocotte test                   Run test files in tests/
cocotte exec <task>            Run a task from Charlotfile
cocotte exec list              List all Charlotfile tasks
cocotte clean                  Remove build artifacts
cocotte package                Package dist/ into a zip
cocotte repl                   Interactive prompt
cocotte disasm <file>          Show bytecode disassembly
```

---

## 23. Complete Example Programs

### Example 1: Calculator

```cocotte
func calculate(a, op, b)
    if op == "+"
        return a + b
    elif op == "-"
        return a - b
    elif op == "*"
        return a * b
    elif op == "/"
        if b == 0
            print "Error: cannot divide by zero"
            return nil
        end
        return divide a by b
    elif op == "%"
        return a % b
    else
        print "Unknown operator: " + op
        return nil
    end
end

print calculate(10, "+", 5)    # 15
print calculate(10, "-", 3)    # 7
print calculate(10, "*", 4)    # 40
print calculate(10, "/", 3)    # 3.333...
print calculate(10, "%", 3)    # 1
```

### Example 2: Word frequency counter

```cocotte
func count_words(text)
    var words = text.lower().split(" ")
    var counts = {}
    for word in words
        var clean = word.trim().replace(",", "").replace(".", "")
        if clean != ""
            if counts.has_key(clean)
                counts.set(clean, counts.get(clean) + 1)
            else
                counts.set(clean, 1)
            end
        end
    end
    return counts
end

var text = "the cat sat on the mat the cat is fat"
var freq = count_words(text)

for word in freq.keys()
    print word + ": " + freq.get(word)
end
```

### Example 3: Sorting algorithms

```cocotte
func bubble_sort(lst)
    var n = lst.len()
    var i = 0
    while i < n - 1
        var j = 0
        while j < n - i - 1
            if lst.get(j) > lst.get(j + 1)
                var temp = lst.get(j)
                lst.set(j, lst.get(j + 1))
                lst.set(j + 1, temp)
            end
            j = j + 1
        end
        i = i + 1
    end
    return lst
end

# Wait — Cocotte lists don't have .set(index, value) yet,
# so use the functional approach instead:
func insertion_sort(lst)
    var sorted = []
    for item in lst
        var placed = false
        var i = 0
        while i < sorted.len()
            if item < sorted.get(i)
                var before = sorted.slice(0, i)
                var after  = sorted.slice(i, sorted.len())
                sorted = before
                sorted.push(item)
                sorted.extend(after)
                placed = true
                break
            end
            i = i + 1
        end
        if not placed
            sorted.push(item)
        end
    end
    return sorted
end

var nums = [5, 2, 8, 1, 9, 3, 7, 4, 6]
print insertion_sort(nums).join(", ")    # 1, 2, 3, 4, 5, 6, 7, 8, 9
```

### Example 4: Simple CSV reader

```cocotte
func parse_csv(text)
    var rows = []
    var lines = text.split_lines()
    for line in lines
        if not line.trim().is_empty()
            rows.push(line.split(",").map(func(cell) return cell.trim() end))
        end
    end
    return rows
end

var csv = "name, age, city\nAlice, 9, Paris\nBob, 10, London\nClara, 8, Tokyo"
var table = parse_csv(csv)

var headers = table.get(0)
var i = 1
while i < table.len()
    var row = table.get(i)
    var j = 0
    while j < headers.len()
        print headers.get(j) + ": " + row.get(j)
        j = j + 1
    end
    print "---"
    i = i + 1
end
```

### Example 5: Note-taking CLI app

```cocotte
module add "json"

var notes_file = "notes.json"

func load_notes()
    if not file_exists(notes_file)
        return []
    end
    try
        return json.parse(read_file(notes_file))
    catch err
        return []
    end
end

func save_notes(notes)
    write_file(notes_file, json.stringify(notes))
end

func add_note(text)
    var notes = load_notes()
    var note = {
        "id": notes.len() + 1,
        "text": text,
        "time": time_now()
    }
    notes.push(note)
    save_notes(notes)
    print "Note saved! (#" + note.get("id") + ")"
end

func list_notes()
    var notes = load_notes()
    if notes.len() == 0
        print "No notes yet."
        return nil
    end
    print "Your notes:"
    for note in notes
        print "  [" + note.get("id") + "] " + note.get("text")
    end
end

func delete_note(id)
    var notes = load_notes()
    var kept = notes.filter(func(n) return n.get("id") != id end)
    save_notes(kept)
    print "Note #" + id + " deleted."
end

# Simple CLI
var cmd = input("Command (add/list/delete/quit): ")
while cmd != "quit"
    if cmd == "add"
        var text = input("Note text: ")
        add_note(text)
    elif cmd == "list"
        list_notes()
    elif cmd == "delete"
        var id = to_number(input("Note ID to delete: "))
        delete_note(id)
    else
        print "Unknown command. Try: add, list, delete, quit"
    end
    cmd = input("Command: ")
end
print "Goodbye!"
```

### Example 6: GUI to-do app

```cocotte
module add "charlotte"

var state = {
    "todos": [],
    "done":  [],
    "input": ""
}

charlotte.window("To-Do App", 500, 600, func(ui)
    ui.heading("My To-Do List")
    ui.separator()

    # Input row
    ui.row(func()
        var text = ui.input("new_todo", "What do you need to do?")
        state.set("input", text)
        if ui.button("Add")
            var item = state.get("input").trim()
            if item != ""
                state.get("todos").push(item)
            end
        end
    end)

    ui.separator()

    # Todo items
    var todos = state.get("todos").copy()
    ui.label("Pending (" + todos.len() + "):")
    var i = 0
    while i < todos.len()
        var item = todos.get(i)
        ui.row(func()
            if ui.small_button("Done")
                state.get("done").push(item)
                var new_todos = state.get("todos").filter(func(t) return t != item end)
                state.set("todos", new_todos)
            end
            ui.label(item)
        end)
        i = i + 1
    end

    ui.separator()

    # Done items
    var done = state.get("done")
    ui.label("Completed (" + done.len() + "):")
    for item in done
        ui.colored_label("gray", "  [x] " + item)
    end

    if done.len() > 0
        if ui.small_button("Clear completed")
            state.set("done", [])
        end
    end
end)
```

### Example 7: Testing your code

Create `tests/math_test.cot`:

```cocotte
# Tests for math utilities
library add "libraries/math_utils.cotlib"

assert_eq(math_utils.square(4), 16)
assert_eq(math_utils.square(0), 0)
assert_eq(math_utils.cube(3), 27)
assert_eq(math_utils.is_even(4), true)
assert_eq(math_utils.is_even(7), false)
assert_eq(math_utils.is_prime(17), true)
assert_eq(math_utils.is_prime(4), false)
assert_eq(math_utils.is_prime(2), true)

print "All math tests passed!"
```

Run with:
```
cocotte test
```

---

## Quick Reference Card

```
# Variables
var x = 42
var name = "Alice"
var done = false
var items = [1, 2, 3]
var data = {"key": "value"}

# Print
print "Hello, " + name

# Math
x + y    x - y    x * y    divide x by y    x % y

# Compare
==    !=    <    >    <=    >=    and    or    not

# If
if x > 5
    ...
elif x == 5
    ...
else
    ...
end

# While
while x < 10
    x = x + 1
end

# For
for item in list
    print item
end

# Function
func name(a, b)
    return a + b
end

# Lambda
var f = func(x) return x * 2 end

# Class
class Name
    func init(x)
        self.x = x
    end
    func method()
        return self.x
    end
end
var obj = Name(42)
obj.method()

# Error handling
try
    ...risky code...
catch err
    print "Error: " + err
end

# Modules
module add "json"
module add "math"
module add "os"
module add "charlotte"

# Libraries
library add "libraries/mylib.cotlib"
```
