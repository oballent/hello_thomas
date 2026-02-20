struct TrainCar {
    id: u32,
    contents: String,
    is_locked: bool,
}




fn main() {
let mut car_1 = TrainCar {
        id: 101,
        contents: String::from("Civilians"), // Bullet Train consequences...
        is_locked: true,
    };

    println!("Car {} contains {}.", car_1.id, car_1.contents);}
