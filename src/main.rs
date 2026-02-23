// This program demonstrates the concept of mutable references in Rust using a simple example of train engines and their personalities.
enum EngineType {
    Diesel,
    Thomas,
    Percy,
}

fn main() {


//let beckett = &mut EngineType::Diesel;
let mut beckett: EngineType = EngineType::Diesel;

//What does it really mean to have keep ownership in main? It means that we can create a mutable variable that holds the engine type, and we can pass a mutable reference to it when we want to rehabilitate its personality. This way, we can modify the engine's personality without losing ownership of the variable in main. But we can also pass a reference to the variable when we want to describe its personality, without needing to modify it. This allows us to keep ownership of the variable in main while still being able to interact with it in different ways.
println!("Beckett's personality: {}", describe_personality(&beckett));

rehabilitate(&mut beckett);

println!("Beckett's personality after rehabilitation: {}", describe_personality(&beckett));


}

fn describe_personality(engine: &EngineType) -> String{
    match engine {
        EngineType::Diesel => String::from("Diesel is a troublemaker, always causing mischief and chaos on the tracks."),
        EngineType::Thomas => String::from("Thomas is a friendly and helpful engine, always ready to lend a hand and make friends."),
        EngineType::Percy => String::from("Percy is a brave and intuitive little engine that doesn't always think things through, but always tries his best."),
    }
}

fn rehabilitate(engine: &mut EngineType) {
    println!("Rehabilitating the engine's personality...");
    // This function would contain logic to rehabilitate the engine's personality
    // For example, if it's a Diesel, we could change it to a Thomas
    *engine = EngineType::Thomas;
}