// This program demonstrates the concept of mutable references in Rust using a simple example of train engines and their personalities.
struct TrainCar {
    id: u32,
    engine: EngineType,
    passenger: Option<String>,
}

impl TrainCar {
    fn rehabilitate(&mut self) {
        println!("Rehabilitating the train car's engine...");
        self.engine = EngineType::Thomas;
    }
    fn check_passenger(&self) {
        match &self.passenger {
            Some(name) => println!("{} is aboard!", name),
            None => println!("Ain't nobody on this train car!"),
        }
    }
}

enum EngineType {
    Diesel,
    Thomas,
    Percy,
}

fn main() {
/*
let mut car_7 = TrainCar{
    id: 7,
    engine: EngineType::Diesel,
};

println!("Car 7's engine personality: {}", describe_personality(&car_7.engine));

car_7.rehabilitate();

println!("Car 7's engine personality after rehabilitation: {}", describe_personality(&car_7.engine));*/



//let beckett = &mut EngineType::Diesel;
let mut beckett: TrainCar = TrainCar{
    id: 7,
    engine: EngineType::Diesel,
    passenger: Some(String::from("Lemon")),
};

let mut diesel_himself = TrainCar{
    id: 8,
    engine: EngineType::Diesel,
    passenger: None,
};

//What does it really mean to have keep ownership in main? It means that we can create a mutable variable that holds the engine type, and we can pass a mutable reference to it when we want to rehabilitate its personality. This way, we can modify the engine's personality without losing ownership of the variable in main. But we can also pass a reference to the variable when we want to describe its personality, without needing to modify it. This allows us to keep ownership of the variable in main while still being able to interact with it in different ways.
println!("Beckett's personality: {}", describe_personality(&beckett.engine));

beckett.rehabilitate();
println!("Beckett's personality after rehabilitation: {}", describe_personality(&beckett.engine));


beckett.check_passenger();
diesel_himself.check_passenger();

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