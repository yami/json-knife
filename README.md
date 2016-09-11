# jk - Json Knife


## Development Stage
This project is in its very early stage, a couple of things missing or unstable are
  1. no pretty printing yet
  2. error reporting is weak
  3. query syntax is unstable
  4. action syntax is unstable
  5. and more...


## What is it?
Jk is a tool to transform Json data to tabular form, inspired by [Micha](https://github.com/micha)'s [json-table](https://github.com/micha/json-table), [jsawk](https://github.com/micha/jsawk) and Goessner's [JsonPath](http://goessner.net/articles/JsonPath/). While Jk's goal is same as json-table's, jk tries to be more intuitive and powerful than json-table. The main difference is that jk adopts a similar form of awk, which makes generating tabular output easier.


## How to use?
A jk program consists of three parts:
```
  <selector> <mode> <action>
```
where
  - `selector` is a query string to select interested parts of a Json input, calling it a `sub-json`
  - `mode` is how to interpret the selected sub-json
  - `action` is a list of commands to run over the selected sub-sjon.

Following are some examples with comments after `#`. First let's print out the json under experiment:
```bash
$ cat store.json
{ "store": {                                    
    "book": [                                   
      { "category": "reference",                
        "author": "Nigel Rees",                 
        "title": "Sayings of the Century",      
        "price": 8.95                           
      },                                        
      { "category": "fiction",                  
        "author": "Evelyn Waugh",               
        "title": "Sword of Honour",             
        "price": 12.99                          
      },                                        
      { "category": "fiction",                  
        "author": "Herman Melville",            
        "title": "Moby Dick",                   
        "isbn": "0-553-21311-3",                
        "price": 8.99                           
      },                                        
      { "category": "fiction",                  
        "author": "J. R. R. Tolkien",           
        "title": "The Lord of the Rings",       
        "isbn": "0-395-19395-8",                
        "price": 22.99                          
      }                                         
    ],                                          
    "bicycle": {                                
      "color": "red",                           
      "price": 19.95                            
    }                                           
  }                                             
}                                               
```
To print authors of each book:
```bash
# selector is `store.book`
# mode is '%' (ForEach), meaning for each array element or object key-value pair
# action is 'p .author' where
#    - 'p' is the command for print
#    - '.author' is the argument to p, which is each  book's author.
$ cat store.json | jk "store.book % p .author"
Nigel Rees
Evelyn Waugh
Herman Melville
J. R. R. Tolkien
```
To print all properties for first book
```bash
# selector is 'store.book.[0]', which selects the first book
# mode is still 'ForEach'.
# action is to print each key-value. Note '_k' and '_v' are builtin-variables.
$ cat store.json | jk "store.book.[0] % p _k _v"
author Nigel Rees
category reference
price 8.95
title Sayings of the Century
```
To print the bicyle color:
```bash
# mode is '@', ForSelf, that is not run action over each elements/key-values.
# Instead, action is run over current sub-json itself.
$ cat store.json | jk "store.bicycle @ p .color"
red
```

## License
MIT