with open("config/src/config.rs", "r") as f:
    content = f.read()

content = content.replace(" *c == ", " c == ")
content = content.replace("ident.push(*c);", "ident.push(c);")
content = content.replace("num_str.push(*c);", "num_str.push(c);")

with open("config/src/config.rs", "w") as f:
    f.write(content)
