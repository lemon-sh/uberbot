// TODO: port leek @karx

macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
        let mut map = ::std::collections::HashMap::new();
        $( map.insert($key, $val); )*
        map
    }}
}

// macro_rules! mock {
//     ($target:expr) => {{
//         let mut builder = String::from("");

//         for char in $target.chars() {
//             if rand::random() {
//                 builder.push_str(&char.to_uppercase().to_string());
//             } else {
//                 builder.push_str(&char.to_lowercase().to_string());
//             }
//         }

//         builder
//     }}
// }

pub fn leetify(target: &str) -> String {
    let letters = hashmap! {
        'a' => '4',
        'e' => '3',
        'i' => '1',
        'o' => '0',
        'g' => '6',
        's' => '5',
        't' => '7',
        'b' => '8'
    };

    let mut builder = String::with_capacity(target.len());

    for char in target.chars() {
        if let Some(repl) = letters.get(&char.to_ascii_lowercase()) {
            builder.push(*repl);
        } else {
            builder.push(char);
        }
    }

    builder
}