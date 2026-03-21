# Cocotte Standard Library — Module Reference

**68 built-in modules** available without installation.

Usage: `library add "stdlib/<module>/module.cotlib"`

---

## Table of Contents

args · assert · base64 · cache · cli · clipboard · collections · color_utils ·
colors · complex · config · crypto · csv · dates · db · deque · docker · dotenv ·
env · events · fmt · fs · functional · geometry · git · graph · hash · heap ·
html · i18n · image · ini · iter · json_schema · logging · markdown · matrices ·
middleware · net · notify · passwords · path · pdf · pipeline · process · queue ·
random · rate_limit · regex · router · scheduler · search · set · sort · stack ·
state · statistics · strings · systeminfo · template · terminal · test · text ·
time · units · url · uuid · validation

---

## args
Command-line argument parsing helpers.
```cocotte
library add "stdlib/args/module.cotlib"
print args.from_env("PORT", "3000")
```
**Functions:** `parse_env_style(str)`, `from_env(name, default)`, `print_help(prog, desc, spec)`

---

## assert
Named assertion helpers for tests and scripts.
```cocotte
library add "stdlib/assert/module.cotlib"
assert.eq(1 + 1, 2, "math works")
```
**Functions:** `eq(a,b,msg)`, `neq(a,b,msg)`, `gt(a,b,msg)`, `lt(a,b,msg)`, `is_true(val,msg)`, `is_false(val,msg)`, `is_nil(val,msg)`, `not_nil(val,msg)`, `contains(lst,item,msg)`, `in_range(val,lo,hi,msg)`

---

## base64
Base64 encoding and decoding via system tools.
```cocotte
library add "stdlib/base64/module.cotlib"
print base64.encode("Hello, World!")
```
**Functions:** `encode(str)`, `decode(str)`, `encode_file(path)`

---

## cache
LRU (Least Recently Used) cache with configurable capacity.
```cocotte
library add "stdlib/cache/module.cotlib"
var c = cache.create(100)
c.set("key", "value")
print c.get("key")
```
**Class `LRUCache`:** `get(key)`, `set(key, val)`, `has(key)`, `size()`, `clear()`, `keys()`
**Functions:** `create(capacity)`

---

## cli
Command-line interface utilities: flag parsing, prompts, progress bars.
```cocotte
library add "stdlib/cli/module.cotlib"
var opts = cli.parse("--port=9192 --debug")
print cli.flag(opts, "port", "8080")
```
**Functions:** `parse(args_str)`, `flag(parsed,name,default)`, `arg(parsed,index,default)`, `usage(prog,desc,options)`, `confirm(prompt)`, `prompt_required(prompt)`, `progress_bar(label,progress,width)`

---

## clipboard
System clipboard read/write via xclip/xsel/pbcopy.
```cocotte
library add "stdlib/clipboard/module.cotlib"
clipboard.copy("Hello!")
print clipboard.paste()
```
**Functions:** `copy(text)`, `paste()`, `clear()`

---

## collections
High-level data structure helpers: Counter, DefaultMap, and list operations.
```cocotte
library add "stdlib/collections/module.cotlib"
var c = collections.counter()
c.add("apple")  c.add("apple")  c.add("banana")
print c.count("apple")   # 2
```
**Classes:** `Counter`, `DefaultMap`
**Functions:** `counter()`, `default_map(default)`, `group_by(lst,fn)`, `chunk(lst,n)`, `flatten(lst)`, `zip(a,b)`, `unique(lst)`, `partition(lst,pred)`

---

## color_utils
Color conversion: hex↔RGB, HSL, lighten, darken, mix.
```cocotte
library add "stdlib/color_utils/module.cotlib"
var rgb = color_utils.hex_to_rgb("#ff5733")
print color_utils.lighten("#ff5733", 0.2)
```
**Functions:** `hex_to_rgb(hex)`, `rgb_to_hex(rgb)`, `rgb_to_hsl(rgb)`, `lighten(hex,amt)`, `darken(hex,amt)`, `mix(a,b,ratio)`

---

## colors
ANSI terminal color codes for styled output.
```cocotte
library add "stdlib/colors/module.cotlib"
print colors.green("Success!")
print colors.red("Error!")
```
**Functions:** `red(s)`, `green(s)`, `yellow(s)`, `blue(s)`, `magenta(s)`, `cyan(s)`, `bold(s)`, `underline(s)`, `success(s)`, `error(s)`, `warning(s)`, `info(s)`, `strip(s)`

---

## complex
Complex number arithmetic (add, sub, mul, div, abs, conjugate).
```cocotte
library add "stdlib/complex/module.cotlib"
var z = complex.new(3, 4)
print z.abs()   # 5
```
**Class `Complex`:** `add(other)`, `sub(other)`, `mul(other)`, `div(other)`, `abs()`, `conjugate()`, `to_string()`
**Functions:** `new(real,imag)`, `from_polar(r,theta)`

---

## config
JSON-based configuration with defaults and save/load.
```cocotte
library add "stdlib/config/module.cotlib"
var cfg = config.load("config.json", {"port": 8080, "debug": false})
print cfg.get("port", 8080)
cfg.set("debug", true)
cfg.save()
```
**Class `Config`:** `get(key,default)`, `set(key,val)`, `save()`, `all()`, `reset(defaults)`
**Functions:** `load(path, defaults)`

---

## crypto
Cryptographic utilities via OpenSSL: AES-256 encrypt/decrypt, HMAC-SHA256.
```cocotte
library add "stdlib/crypto/module.cotlib"
var enc = crypto.encrypt("secret", "mypassword")
print crypto.decrypt(enc, "mypassword")
```
**Functions:** `encrypt(text,password)`, `decrypt(ciphertext,password)`, `random_bytes(n)`, `hmac_sha256(msg,key)`, `gen_self_signed_cert(key_path,cert_path,days)`

---

## csv
CSV parsing and writing with header support.
```cocotte
library add "stdlib/csv/module.cotlib"
var rows = csv.read("data.csv", ",")
for row in rows  print row.get("name")  end
csv.write("out.csv", rows, ",")
```
**Functions:** `parse(text,sep)`, `parse_with_header(text,sep)`, `read(path,sep)`, `stringify(rows,sep)`, `write(path,rows,sep)`

---

## dates
Date arithmetic: add days, diff days, format, parse ISO dates.
```cocotte
library add "stdlib/dates/module.cotlib"
var today = dates.today()
var tomorrow = dates.add_days(today, 1)
print dates.format_iso(tomorrow)
```
**Functions:** `today()`, `format_iso(date)`, `format_long(date)`, `parse_iso(str)`, `add_days(date,n)`, `diff_days(a,b)`, `is_leap(year)`, `days_in_month(month,year)`, `month_name(m)`, `day_name(d)`

---

## db
High-level SQLite wrapper with migrations, CRUD helpers.
```cocotte
library add "stdlib/db/module.cotlib"
var db = db.open("app.db")
db.create_table("users", ["id INTEGER PRIMARY KEY AUTOINCREMENT", "name TEXT"])
db.insert("users", {"name": "Alice"})
print db.count("users", "1=1")
```
**Class `DB`:** `exec(sql)`, `query(sql)`, `query_one(sql)`, `create_table(name,cols)`, `insert(table,data)`, `update(table,data,where)`, `delete(table,where)`, `find(table,where)`, `find_one(table,where)`, `count(table,where)`, `exists(table,where)`, `migrate(migrations)`, `tables()`
**Functions:** `open(path)`

---

## deque
Double-ended queue (push/pop from both ends).
```cocotte
library add "stdlib/deque/module.cotlib"
var d = deque.create()
d.push_front("a")  d.push_back("b")
print d.pop_front()   # a
```
**Class `Deque`:** `push_front(item)`, `push_back(item)`, `pop_front()`, `pop_back()`, `front()`, `back()`, `size()`, `is_empty()`
**Functions:** `create()`

---

## docker
Docker management: containers, images, build, compose.
```cocotte
library add "stdlib/docker/module.cotlib"
print docker.is_running()
print docker.containers(false).join(", ")
```
**Functions:** `is_running()`, `containers(all)`, `images()`, `run_container(image,name,opts)`, `stop(c)`, `start(c)`, `remove_container(c,force)`, `exec_cmd(c,cmd)`, `logs(c,lines)`, `build(tag,path)`, `compose_up(detach)`, `compose_down()`

---

## dotenv
Load `.env` files into a map for configuration.
```cocotte
library add "stdlib/dotenv/module.cotlib"
var env = dotenv.load(".env")
print env.get("DATABASE_URL")
```
**Functions:** `load(path)`, `get(path,key,default)`, `save(path,map)`

---

## env
Environment variable utilities with type coercion and validation.
```cocotte
library add "stdlib/env/module.cotlib"
var port = env.get_number("PORT", 8080)
var debug = env.get_bool("DEBUG", false)
```
**Functions:** `get(name,default)`, `get_number(name,default)`, `get_bool(name,default)`, `is_set(name)`, `require(name)`, `info()`

---

## events
Synchronous event emitter (on, once, off, emit).
```cocotte
library add "stdlib/events/module.cotlib"
var emitter = events.create()
emitter.on("data", func(d) print "got: " + d end)
emitter.emit("data", "hello")
```
**Class `EventEmitter`:** `on(event,fn)`, `once(event,fn)`, `off(event,fn)`, `emit(event,data)`, `count(event)`, `clear()`
**Functions:** `create()`

---

## fmt
String formatting: padding, thousands, bytes, duration, truncate, slugify.
```cocotte
library add "stdlib/fmt/module.cotlib"
print fmt.thousands(1234567)   # 1,234,567
print fmt.bytes(1048576)       # 1.0 MB
print fmt.duration(3661)       # 1h 1m 1s
```
**Functions:** `pad_left(s,w,ch)`, `pad_right(s,w,ch)`, `center(s,w,ch)`, `thousands(n)`, `bytes(n)`, `truncate(s,max,suffix)`, `repeat(s,n)`, `duration(secs)`, `percent(n,total,decimals)`

---

## fs
Extended filesystem utilities built on top of Cocotte's built-ins.
```cocotte
library add "stdlib/fs/module.cotlib"
var lines = fs.read_lines("data.txt")
fs.write_json("config.json", {"port": 8080})
print fs.extension("photo.jpg")   # jpg
```
**Functions:** `read_lines(path)`, `write_lines(path,lines)`, `append_line(path,line)`, `exists(path)`, `ensure_dir(path)`, `size(path)`, `copy(src,dst)`, `move(src,dst)`, `remove(path)`, `list(dir)`, `list_full(dir)`, `read_json(path)`, `write_json(path,val)`, `extension(path)`, `stem(path)`, `dirname(path)`, `basename(path)`, `write_if_changed(path,content)`

---

## functional
Functional programming: compose, pipe, curry, memoize, partial, flip.
```cocotte
library add "stdlib/functional/module.cotlib"
var double = func(x) return x * 2 end
var inc    = func(x) return x + 1 end
var f = functional.compose(double, inc)
print f(5)   # 12
```
**Functions:** `compose(f,g)`, `pipe(val,fns)`, `curry2(f)`, `partial(f,a)`, `memoize(f)`, `identity(x)`, `constant(val)`, `times(f,n,x)`, `negate(pred)`, `flip(f)`, `safe(f,x)`

---

## geometry
2D/3D geometry: distance, areas, angles, vectors, collision.
```cocotte
library add "stdlib/geometry/module.cotlib"
print geometry.distance_2d(0, 0, 3, 4)   # 5
print geometry.circle_area(7)
```
**Functions:** `distance_2d/3d`, `circle_area/circumference`, `rect_area/perimeter`, `triangle_area`, `to_radians/degrees`, `dot_2d`, `cross_2d`, `normalize_2d`, `lerp`, `point_in_rect`, `circles_overlap`

---

## git
Git repository management via os.exec.
```cocotte
library add "stdlib/git/module.cotlib"
print git.current_branch()
git.commit("fix: typo")
```
**Functions:** `is_repo()`, `current_branch()`, `status()`, `log(n)`, `add(files)`, `commit(msg)`, `push(remote,branch)`, `pull(remote,branch)`, `clone(url,dest)`, `diff()`, `stash()`, `stash_pop()`, `tags()`, `remotes()`, `last_commit_hash()`, `last_commit_message()`

---

## graph
Directed/undirected graphs with BFS and DFS traversal.
```cocotte
library add "stdlib/graph/module.cotlib"
var g = graph.undirected()
g.add_edge("A", "B")  g.add_edge("B", "C")
print graph.bfs("A")
```
**Class `Graph`:** `add_node(n)`, `add_edge(from,to)`, `neighbors(n)`, `has_node(n)`, `nodes()`, `bfs(start)`, `dfs(start)`, `has_path(from,to)`
**Functions:** `directed()`, `undirected()`

---

## hash
Hashing: SHA256, SHA1, MD5, HMAC, DJB2.
```cocotte
library add "stdlib/hash/module.cotlib"
print hash.sha256("hello")
print hash.djb2("hello")
```
**Functions:** `sha256(s)`, `sha1(s)`, `md5(s)`, `sha256_file(path)`, `djb2(s)`, `verify_sha256(s,expected)`

---

## heap
Min-heap (priority queue) implementation.
```cocotte
library add "stdlib/heap/module.cotlib"
var h = heap.create()
h.push(5)  h.push(1)  h.push(3)
print h.pop()   # 1
```
**Class `MinHeap`:** `push(item)`, `pop()`, `peek()`, `size()`, `is_empty()`, `to_list()`
**Functions:** `create()`

---

## html
HTML generation: tags, tables, lists, full page scaffolding.
```cocotte
library add "stdlib/html/module.cotlib"
print html.page("My App", html.h1("Hello") + html.p("World"), nil)
```
**Functions:** `tag(name,content,attrs)`, `h1/h2/h3/p/strong/em/code/pre/span/div(text)`, `a(label,href,attrs)`, `img(src,alt)`, `ul(items)`, `ol(items)`, `table(headers,rows)`, `page(title,body,style)`, `escape(s)`

---

## i18n
Internationalization: locale management, translation lookup, pluralization.
```cocotte
library add "stdlib/i18n/module.cotlib"
i18n.set_locale("fr")
i18n.load("fr", "locales/fr.json")
print i18n.t("welcome")
```
**Functions:** `set_locale(l)`, `get_locale()`, `load(locale,path)`, `t(key)`, `t_vars(key,vars)`, `pluralize(n,singular,plural)`

---

## image
Image processing via ImageMagick.
```cocotte
library add "stdlib/image/module.cotlib"
image.resize("photo.jpg", "thumb.jpg", 128, 128)
image.grayscale("photo.jpg", "bw.jpg")
```
**Functions:** `resize(in,out,w,h)`, `crop(in,out,w,h,x,y)`, `convert(in,out)`, `grayscale(in,out)`, `blur(in,out,r)`, `rotate(in,out,deg)`, `thumbnail(in,out,size)`, `info(path)`, `watermark(in,out,text)`

---

## ini
INI configuration file parser and writer.
```cocotte
library add "stdlib/ini/module.cotlib"
var cfg = ini.read("app.ini")
print ini.get(cfg, "database", "host", "localhost")
```
**Functions:** `parse(text)`, `read(path)`, `get(data,section,key,default)`, `stringify(data)`, `write(path,data)`

---

## iter
Iteration helpers: range_step, enumerate, windows, take_while, cycle.
```cocotte
library add "stdlib/iter/module.cotlib"
for pair in iter.enumerate(["a","b","c"])
    print pair.get(0) + ": " + pair.get(1)
end
```
**Functions:** `range_step(start,stop,step)`, `count(start,max)`, `repeat(val,n)`, `cycle(lst,n)`, `enumerate(lst)`, `windows(lst,n)`, `take_while(lst,pred)`, `drop_while(lst,pred)`, `indexed(lst)`, `pairwise(lst,fn)`

---

## json_schema
JSON schema validation: type, required fields, min/max, nested properties.
```cocotte
library add "stdlib/json_schema/module.cotlib"
var schema = {"type": "map", "required": ["name", "age"]}
var errors = json_schema.validate({"name": "Alice", "age": 30}, schema)
print errors.len() == 0   # true
```
**Functions:** `validate(value,schema)`, `is_valid(value,schema)`

---

## logging
Structured logging with levels (DEBUG/INFO/WARN/ERROR) and file output.
```cocotte
library add "stdlib/logging/module.cotlib"
logging.set_level(1)
logging.info("Server started")
logging.warn("High memory usage")
```
**Functions:** `set_level(n)`, `set_file(path)`, `debug(msg)`, `info(msg)`, `warn(msg)`, `error(msg)`, `tag(t,msg)`, `structured(level,fields)`

---

## markdown
Programmatic Markdown generation: headings, lists, tables, badges, TOC.
```cocotte
library add "stdlib/markdown/module.cotlib"
var doc = markdown.h1("My Project") + markdown.ul(["feature 1","feature 2"])
write_file("README.md", doc)
```
**Functions:** `h1/h2/h3/h4(text)`, `bold(text)`, `italic(text)`, `code(text)`, `link(label,url)`, `image(alt,url)`, `blockquote(text)`, `code_block(code,lang)`, `ul(items)`, `ol(items)`, `table(headers,rows)`, `badge(label,val,color)`, `toc(headings)`

---

## matrices
2D matrix operations: add, scale, transpose, multiply, flatten.
```cocotte
library add "stdlib/matrices/module.cotlib"
var a = matrices.zeros(3, 3)
var b = matrices.identity(3)
matrices.print_mat(matrices.add(a, b))
```
**Functions:** `zeros(m,n)`, `identity(n)`, `get(mat,i,j)`, `rows(mat)`, `cols(mat)`, `add(a,b)`, `scale(mat,s)`, `transpose(mat)`, `multiply(a,b)`, `flatten(mat)`, `print_mat(mat)`

---

## middleware
HTTP middleware: CORS, logging, auth, rate limiting, composition.
```cocotte
library add "stdlib/middleware/module.cotlib"
var handler = middleware.cors(my_handler, "*")
handler = middleware.logger(handler)
http.serve(9192, handler)
```
**Functions:** `cors(handler,origins)`, `logger(handler)`, `auth_bearer(handler,token)`, `rate_limit(handler,rps)`, `compose(handler,middlewares)`

---

## net
Network utilities: reachability, HTTP wrappers, DNS, download.
```cocotte
library add "stdlib/net/module.cotlib"
print net.public_ip()
net.download("https://example.com/file.txt", "/tmp/file.txt")
```
**Functions:** `is_reachable(host)`, `get(url)`, `post(url,body)`, `public_ip()`, `dns_lookup(host)`, `download(url,path)`

---

## notify
Desktop notifications via notify-send / osascript.
```cocotte
library add "stdlib/notify/module.cotlib"
notify.info("Build", "Compilation complete!")
```
**Functions:** `send(title,msg,urgency)`, `info(title,msg)`, `warn(title,msg)`, `error(title,msg)`, `bell()`

---

## passwords
Password generation, strength scoring, passphrase generation.
```cocotte
library add "stdlib/passwords/module.cotlib"
print passwords.generate(16, true)
print passwords.strength_label("Abc123!")   # Weak
```
**Functions:** `generate(length,use_symbols)`, `strength(pwd)`, `strength_label(pwd)`, `passphrase(words,count,sep)`

---

## path
Path manipulation: join, basename, dirname, stem, ext, normalize, home.
```cocotte
library add "stdlib/path/module.cotlib"
print path.join("/home/user", "documents")
print path.ext("photo.jpg")   # jpg
print path.expand_home("~/docs")
```
**Functions:** `join(a,b)`, `join_many(parts)`, `ext(path)`, `basename(path)`, `dirname(path)`, `stem(path)`, `normalize(path)`, `is_absolute(path)`, `is_relative(path)`, `split(path)`, `with_ext(path,ext)`, `home()`, `expand_home(path)`

---

## pdf
PDF operations via poppler-utils and pandoc.
```cocotte
library add "stdlib/pdf/module.cotlib"
print pdf.pages("document.pdf")
var text = pdf.to_text("document.pdf")
```
**Functions:** `pages(path)`, `to_text(path)`, `to_text_page(path,page)`, `merge(inputs,output)`, `html_to_pdf(html,out)`, `markdown_to_pdf(md,out)`, `compress(input,output)`

---

## pipeline
Data processing pipeline builder with error handling.
```cocotte
library add "stdlib/pipeline/module.cotlib"
var p = pipeline.create()
    .pipe(func(x) return x * 2 end)
    .pipe(func(x) return x + 1 end)
print p.run(5)   # 11
```
**Class `Pipeline`:** `pipe(fn)`, `on_error(fn)`, `run(value)`, `run_all(items)`
**Functions:** `create()`

---

## process
Process management: run commands, check if running, find executables.
```cocotte
library add "stdlib/process/module.cotlib"
var lines = process.run_lines("ls -la")
print process.command_exists("git")
```
**Functions:** `run(cmd)`, `run_lines(cmd)`, `pid()`, `kill(pid)`, `is_running(name)`, `list_processes()`, `which(cmd)`, `command_exists(cmd)`, `shell(cmd,shell_path)`

---

## queue
FIFO queue data structure.
```cocotte
library add "stdlib/queue/module.cotlib"
var q = queue.create()
q.enqueue("first")  q.enqueue("second")
print q.dequeue()   # first
```
**Class `Queue`:** `enqueue(item)`, `dequeue()`, `front()`, `back()`, `is_empty()`, `size()`
**Functions:** `create()`

---

## random
Extended random: floats, integers, booleans, choice, shuffle, weighted.
```cocotte
library add "stdlib/random/module.cotlib"
print random.int(1, 6)
var shuffled = random.shuffle([1, 2, 3, 4, 5])
print random.choice(["a","b","c"])
```
**Functions:** `float(min,max)`, `int(min,max)`, `bool_val(p)`, `choice(lst)`, `sample(lst,n)`, `shuffle(lst)`, `string(len,charset)`, `hex(len)`, `weighted_choice(items,weights)`

---

## rate_limit
Token bucket rate limiter for controlling request frequency.
```cocotte
library add "stdlib/rate_limit/module.cotlib"
var limiter = rate_limit.create(10, 5)   # 10 tokens, 5/sec refill
if limiter.allow()  print "request allowed" end
```
**Class `RateLimiter`:** `allow()`, `wait_and_allow()`, `available_tokens()`
**Functions:** `create(max_tokens, refill_per_sec)`

---

## regex
Regular expression matching and replacement via system grep/sed.
```cocotte
library add "stdlib/regex/module.cotlib"
print regex.matches("hello@world.com", "^[\\w.]+@[\\w.]+$")
print regex.find_all("one 1 two 2 three 3", "[0-9]+")
```
**Functions:** `matches(s,pattern)`, `find_all(s,pattern)`, `replace_first(s,pat,rep)`, `replace_all(s,pat,rep)`, `split(s,pattern)`, `extract(s,pattern)`, `is_email(s)`, `is_url(s)`, `is_ipv4(s)`

---

## router
HTTP request router with path parameter support for use with `http.serve`.
```cocotte
library add "stdlib/router/module.cotlib"
var r = router.create()
r.get("/api/users", func(req, params) return {"status":200,"body":"[]"} end)
r.get("/api/users/:id", func(req, params) return {"status":200,"body":params.get("id")} end)
router.serve(9192, r)
```
**Class `Router`:** `get/post/put/delete/any(path,handler)`, `not_found(handler)`, `handle(req)`
**Functions:** `create()`, `serve(port, router)`

---

## scheduler
Polling-based task scheduler: run tasks every N seconds or once after delay.
```cocotte
library add "stdlib/scheduler/module.cotlib"
var s = scheduler.create()
s.every(60, "backup", func() print "backing up..." end)
s.run(1, nil)   # run forever with 1s tick
```
**Class `Scheduler`:** `every(secs,name,fn)`, `after(secs,name,fn)`, `run(tick,max_iter)`, `stop()`, `task_count()`
**Functions:** `create()`

---

## search
Search algorithms: linear, binary, full-text, fuzzy.
```cocotte
library add "stdlib/search/module.cotlib"
var users = [{"name":"Alice"}, {"name":"Bob"}]
print search.fulltext(users, "ali")
```
**Functions:** `linear(lst,target)`, `binary(sorted_lst,target)`, `fulltext(items,query)`, `fuzzy(items,query,field)`

---

## set
Unique-element set with union, intersection, difference.
```cocotte
library add "stdlib/set/module.cotlib"
var s = set.from_list([1,2,3,2,1])
print s.size()   # 3
```
**Class `Set`:** `add(item)`, `remove(item)`, `has(item)`, `size()`, `is_empty()`, `to_list()`, `union(other)`, `intersection(other)`, `difference(other)`
**Functions:** `create()`, `from_list(lst)`

---

## sort
Sorting algorithms and key-function sorting.
```cocotte
library add "stdlib/sort/module.cotlib"
var users = [{"name":"Bob","age":30}, {"name":"Alice","age":25}]
var sorted = sort.by_field(users, "name")
```
**Functions:** `by(lst,key_fn)`, `by_desc(lst,key_fn)`, `by_field(lst,field)`, `by_field_desc(lst,field)`, `binary_search(lst,target)`, `merge_sort(lst)`

---

## stack
LIFO stack data structure.
```cocotte
library add "stdlib/stack/module.cotlib"
var s = stack.create()
s.push(1)  s.push(2)
print s.pop()   # 2
```
**Class `Stack`:** `push(item)`, `pop()`, `peek()`, `is_empty()`, `size()`, `clear()`, `to_list()`
**Functions:** `create()`

---

## state
Reactive state store with subscription, history, and undo.
```cocotte
library add "stdlib/state/module.cotlib"
var store = state.create({"count": 0})
store.subscribe(func(s) print "count=" + s.get("count") end)
store.update("count", 1)
```
**Class `Store`:** `get_state()`, `set(new_state)`, `update(key,val)`, `subscribe(fn)`, `undo()`, `reset(initial)`
**Functions:** `create(initial)`

---

## statistics
Statistical functions: mean, median, mode, std_dev, percentile, normalize.
```cocotte
library add "stdlib/statistics/module.cotlib"
var data = [1, 2, 3, 4, 5]
print statistics.mean(data)     # 3
print statistics.std_dev(data)
var summary = statistics.describe(data)
```
**Functions:** `sum(lst)`, `mean(lst)`, `median(lst)`, `mode(lst)`, `variance(lst)`, `std_dev(lst)`, `min_val(lst)`, `max_val(lst)`, `range_val(lst)`, `percentile(lst,p)`, `describe(lst)`, `normalize(lst)`

---

## strings
Extended string utilities: reverse, palindrome, camelCase, snake_case, slug, edit distance.
```cocotte
library add "stdlib/strings/module.cotlib"
print strings.title_case("hello world")   # Hello World
print strings.slugify("Hello World!")     # hello-world
print strings.edit_distance("kitten","sitting")   # 3
```
**Functions:** `count(s,sub)`, `reverse(s)`, `is_palindrome(s)`, `title_case(s)`, `to_camel_case(s)`, `to_snake_case(s)`, `compact(s)`, `edit_distance(a,b)`, `word_wrap(s,width)`, `word_count(s)`, `slugify(s)`

---

## systeminfo
System info: hostname, CPU count, memory, disk, uptime, OS version.
```cocotte
library add "stdlib/systeminfo/module.cotlib"
var info = systeminfo.info()
print info.get("hostname")
print systeminfo.total_memory_mb()
```
**Functions:** `platform()`, `hostname()`, `username()`, `cpu_count()`, `total_memory_mb()`, `free_memory_mb()`, `disk_free_gb(path)`, `uptime_seconds()`, `cpu_usage_percent()`, `os_version()`, `info()`

---

## template
String templating with `{{key}}` substitution and conditional blocks.
```cocotte
library add "stdlib/template/module.cotlib"
print template.render("Hello, {{name}}!", {"name": "Alice"})
```
**Functions:** `render(tmpl,vars)`, `render_file(path,vars)`, `render_to_file(tmpl,vars,out)`, `render_if(tmpl,vars)`

---

## terminal
Terminal control: clear, width/height, box, spinner, cursor movement.
```cocotte
library add "stdlib/terminal/module.cotlib"
terminal.box("Status", "All systems operational")
terminal.rule("─")
```
**Functions:** `clear()`, `width()`, `height()`, `rule(char)`, `center(text)`, `move_cursor(row,col)`, `hide_cursor()`, `show_cursor()`, `box(title,content)`, `spinner(label)`

---

## test
Unit testing framework with suite/it/expect assertions.
```cocotte
library add "stdlib/test/module.cotlib"
test.suite("Math")
test.it("adds correctly", func()
    test.expect_eq(1 + 1, 2)
end)
test.summary()
```
**Functions:** `suite(name)`, `it(desc,fn)`, `expect_eq(actual,expected)`, `expect_neq`, `expect_true`, `expect_false`, `expect_nil`, `expect_not_nil`, `expect_contains`, `expect_throws`, `expect_in_range`, `summary()`, `reset()`

---

## text
Text processing: word extraction, frequency, indent, dedent, grep, summarize.
```cocotte
library add "stdlib/text/module.cotlib"
print text.word_count("hello world foo")   # 3
print text.indent("line 1\nline 2", 4)
```
**Functions:** `line_count(s)`, `words(s)`, `frequency(s)`, `indent(s,n)`, `dedent(s)`, `grep(s,prefix)`, `summarize(s,n)`

---

## time
Time utilities: now, sleep_ms, elapsed, format, stopwatch, measure.
```cocotte
library add "stdlib/time/module.cotlib"
var sw = time.Stopwatch()
sleep(1)
print sw.elapsed_secs()
print time.now_str()
```
**Functions:** `now()`, `sleep_ms(ms)`, `elapsed(start)`, `format(ts)`, `now_str()`, `today()`, `year()`, `month()`, `day()`, `measure(fn)`
**Class `Stopwatch`:** `stop()`, `resume()`, `reset()`, `elapsed_secs()`, `elapsed_ms()`

---

## units
Unit conversion: length, weight, temperature, speed, area, volume, data.
```cocotte
library add "stdlib/units/module.cotlib"
print units.km_to_miles(10)         # 6.21371
print units.celsius_to_fahrenheit(100)   # 212
```
**Functions:** Length, weight, temperature, speed, area, volume, data size conversions (40+ functions).

---

## url
URL parsing, building, query string handling, encoding.
```cocotte
library add "stdlib/url/module.cotlib"
var parts = url.parse("https://api.example.com/users?page=2")
print parts.get("host")    # api.example.com
print parts.get("query")   # page=2
```
**Functions:** `parse(url)`, `parse_query(str)`, `build_query(params)`, `join(base,path)`, `encode(s)`

---

## uuid
UUID v4 generation and validation.
```cocotte
library add "stdlib/uuid/module.cotlib"
print uuid.v4()
print uuid.short_id()
print uuid.is_valid("550e8400-e29b-41d4-a716-446655440000")
```
**Functions:** `v4()`, `short_id()`, `is_valid(s)`

---

## validation
Data validation: email, URL, integer, non-empty, range, alphanumeric, map schema.
```cocotte
library add "stdlib/validation/module.cotlib"
print validation.is_email("alice@example.com")   # true
print validation.is_integer("42")               # true
var errors = validation.validate_map(data, {"name":"required","email":"optional"})
```
**Functions:** `is_email(s)`, `is_url(s)`, `is_integer(s)`, `is_non_empty(s)`, `in_range(n,min,max)`, `length_between(s,min,max)`, `is_alphanumeric(s)`, `validate_map(data,schema)`, `clamp_number(n,min,max)`

---

*Generated automatically from Cocotte stdlib source — 68 modules total.*
