use arrayvec::ArrayString;

pub fn mock(target: &str) -> ArrayString<512> {
    let mut builder = ArrayString::<512>::new();

    for char in target.chars() {
        if rand::random() {
            builder.push(char.to_ascii_uppercase());
        } else {
            builder.push(char.to_ascii_lowercase());
        }
    }

    builder
}

pub fn leetify(target: &str) -> ArrayString<512> {
    let mut builder = ArrayString::<512>::new();

    for char in target.chars() {
        builder.push(match char {
            'a' => '4',
            'e' => '3',
            'i' => '1',
            'o' => '0',
            'g' => '6',
            's' => '5',
            't' => '7',
            'b' => '8',
            _ => char,
        });
    }

    builder
}
