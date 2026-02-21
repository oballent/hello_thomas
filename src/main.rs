struct TrainCar {
    id: u32,
    contents: String,
}

fn main() {
    let car = TrainCar {
        id: 7,
        contents: String::from("Diesel"),
    };

    // We pass a reference (&) so we don't 'lose' the car
    inspect_car(&car);

    println!("Car {} is still in the station.", car.id);
}

fn inspect_car(target: &TrainCar) {
    println!("Inspecting car... it contains: {}", target.contents);
}