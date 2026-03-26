use crate::models::{Mission, MissionReport, StationCommand, Train, TrainError, Location};
use crate::facilities::Station;
use std::collections::HashMap;
use std::sync::mpsc::{self, Sender};

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";


pub struct RailwayNetwork {
    // Maps (Origin, Destination) -> Distance in km
    tracks: HashMap<(String, String), f64>,
    // The network also needs to hold the Stations themselves so it can route trains between them
    //stations: HashMap<String, Station>,
    // We no longer hold physical Stations. We hold the Transmitters to their Actor threads!
    station_handles: HashMap<String, Sender<StationCommand>>,
    //missions: HashMap<u32, Mission>, // <-- The Source of Truth for all missions on the network
    station_locations: HashMap<String, Location>
}

impl RailwayNetwork {
    pub fn new() -> Self {
        RailwayNetwork {
            tracks: HashMap::new(),
            //stations: HashMap::new(),
            station_handles: HashMap::new(),
            //missions: HashMap::new(),
            station_locations: HashMap::new(),
        }
    }
    

    pub fn add_station(&mut self, station: Station) {
        // station.name is a String. We can clone it to use as the key, 
        // and move the actual station into the value.
        self.station_handles.insert(station.name.clone(), station.tx);
        self.station_locations.insert(station.name.clone(), station.location.clone());
    }

    pub fn add_tracks(&mut self, origin: &Station, destination: &Station,) {
        // Clone the names to own them inside the tuple key
        let route = (origin.name.clone(), destination.name.clone());
        let distance = origin.location.distance_to(&destination.location);
        self.tracks.insert(route, distance);
        
        // Sodor is not a one-way street!
        let return_route = (destination.name.clone(), origin.name.clone());
        let return_distance = destination.location.distance_to(&origin.location);
        self.tracks.insert(return_route, return_distance);
    }

    // pub fn add_mission(&mut self, mission: Mission) {
    //     println!("{YELLOW}Network Ledger: Registering Mission {}.{RESET}", mission.id);
    //     self.missions.insert(mission.id, mission);
    // }

    pub fn get_distance(&self, origin: &str, destination: &str) -> Option<f64> {
        // We create a temporary tuple of String objects to match the HashMap key signature.
        self.tracks.get(&(origin.to_string(), destination.to_string())).copied()
    }

    // pub fn get_mission(&self, mission_id: &u32) -> Option<&Mission> {
    //     self.missions.get(mission_id)
    // }


pub fn dispatch_train_across_network(&self, mission: Mission) {
        // 1. Get the Mission
        // let mission = match self.missions.get(&mission.id) {
        //     Some(m) => m.clone(), // We clone it so we don't hold a lock on the HashMap
        //     None => {
        //         println!("{RED}Network Error: Mission {} does not exist.{RESET}", mission.id);
        //         return;
        //     }
        // };

        // 2. Get the Distance
        let distance = match self.get_distance(&mission.origin, &mission.destination) {
            Some(d) => d,
            None => {
                println!("{RED}Error: No track laid between {} and {}.{RESET}", mission.origin, mission.destination);
                return;
            }
        };

        // 3. Find the Origin and Destination Radios
        let origin_tx = match self.station_handles.get(&mission.origin) {
            Some(tx) => tx.clone(),
            None => return,
        };
        let dest_tx = match self.station_handles.get(&mission.destination) {
            Some(tx) => tx.clone(),
            None => return,
        };

        // 4. Create the Transit Channel (The radio frequency the Origin will use to send the Train to the Network)
        let (transit_tx, transit_rx) = mpsc::channel::<Result<Train, TrainError>>();

        // 5. Send the command to the Origin!
        println!("{YELLOW}Network: Ordering {} to assemble Mission {}.{RESET}", mission.origin, mission.id);
        let _ = origin_tx.send(StationCommand::AssembleMission {
            mission: mission.clone(),
            distance,
            reply_to: transit_tx,
        });


        //We will now create a conductor thread that will wait for the train to be built and then route it to the destination. This way, the Network can continue to process other missions while this one is in transit.
        std::thread::spawn(move || {
// 6. Block and wait for the Train to be built
            let report = match transit_rx.recv() {
                Ok(Ok(mut train)) => {
                    println!("{GREEN}Network: Train {} received from {}. Routing to {}...{RESET}", train.id, mission.origin, mission.destination);
                    
                    // The Physics Engine: Try to burn fuel!
                    //train.engine.current_fuel = 0.0; // Let's say the train starts with an empty tank to make it interesting. Will it survive the journey?
                    if let Err(e) = train.dispatch() {
                        println!("{RED}Train {} failed during dispatch: {:?}. Towing back to {}.{RESET}", train.id, e, mission.origin);
                        
                        // RECOVERY: The train failed! Send it BACK to the Origin Station!
                        let (recovery_tx, _recovery_rx) = mpsc::channel();
                        let _ = origin_tx.send(StationCommand::ReceiveTrain {
                            train, // The train is safely handed back to the origin
                            reply_to: recovery_tx,
                        });

                        // Evaluate to the Failure report (NO `return` keyword!)
                        MissionReport::Failure(format!("Train failed during dispatch: {:?}", e))
                        
                    } else {
                        // SUCCESS: Train survived the journey! Send it to the destination.
                        let (arrival_tx, arrival_rx) = mpsc::channel::<Result<(), TrainError>>();
                        let _ = dest_tx.send(StationCommand::ReceiveTrain {
                            train,
                            reply_to: arrival_tx.clone(),
                        });

                        // 8. Wait for the Destination to confirm breakdown
                        match arrival_rx.recv() {
                            Ok(Ok(_)) => MissionReport::Success(format!("Train arrived safely at {}", mission.destination)),
                            _ => MissionReport::Failure("Train failed during receiving/disassembly.".to_string()),
                        }
                    }
                },
                Ok(Err(e)) => MissionReport::Failure(format!("Assembly failed at {}: {:?}", mission.origin, e)),
                Err(_) => MissionReport::Failure("Origin station radio died during assembly.".to_string()),
            };

            // 9. Send the final report back to the customer thread that requested it!
            if let Some(reply_tx) = mission.reply_channel {
                let _ = reply_tx.send(report);
            }
        });
    }


}