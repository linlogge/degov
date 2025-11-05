wit_bindgen::generate!({
    // the name of the world in the `*.wit` input file
    world: "host",
});

// Define a custom type and implement the generated `Guest` trait for it which
// represents implementing all the necessary exported interfaces for this
// component.
struct MyHost;

impl Guest for MyHost {
    fn add(x: u32, y: u32) -> u32 {
        x + y
    }
}

// export! defines that the `MyHost` struct defined below is going to define
// the exports of the `world`, namely the `run` function.
export!(MyHost);
