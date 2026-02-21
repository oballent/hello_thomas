struct TrainCar {
    id: u32,
    contents: String,
}

fn main() {
    let car = TrainCar {
        id: 7,
        contents: String::from("Diesel"), // Change this to "Thomas" to see the difference
    };

    // We pass a reference to our "Security Check"
    let safety_status = check_security(&car);
    
    println!("Car {}: {}", car.id, safety_status);
}

fn check_security(car: &TrainCar) -> String {
    // We 'Read' the contents without taking ownership
    if car.contents == "Diesel" {
        String::from("Warning: Troublemaker detected on the rails.")
    } else {
        String::from("All clear. A very useful engine indeed.")
    }
}