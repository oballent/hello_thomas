use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;

// 1. The wrapper to hold a station and its cumulative distance in the queue
#[derive(Clone, PartialEq)]
struct RouteState {
    cost: f64,
    station: String,
}

// 2. We promise the compiler we can check for absolute equality
impl Eq for RouteState {}

// 3. THE MAGIC FLIP: We teach Rust how to compare RouteStates.
// By flipping `other` and `self`, we trick the Max-Heap into acting like a Min-Heap!
impl Ord for RouteState {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for RouteState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}


















use crate::models::{Mission, MissionReport, StationCommand, Train, TrainError, Location};
use crate::facilities::Station;
//use std::collections::HashMap;
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
                Some(d) => {
                    println!(
                        "{YELLOW}The distance is {} km between {} and {}.{RESET}",
                        d, mission.origin, mission.destination
                    );
                    d
                },
                None => {
                    println!("{RED}Error: No track laid between {} and {}.{RESET}", mission.origin, mission.destination);
                    return;
                }
            };

            // 3. Find the Origin and Destination Radios
            let origin_tx = match self.station_handles.get(&mission.origin) {
                Some(tx) =>{
                    println!("{GREEN}Network: Found radio transmitter for origin station {}.{RESET}", mission.origin);
                    tx.clone()
                }
                None => {
                    println!("{RED}Network Error: No radio transmitter found for origin station {}.{RESET}", mission.origin);
                    return;
                }
            };
            let dest_tx = match self.station_handles.get(&mission.destination) {
                Some(tx) => {
                    println!("{GREEN}Network: Found radio transmitter for destination station {}.{RESET}", mission.destination);
                    tx.clone()
                }

                None => {
                    println!("{RED}Network Error: No radio transmitter found for destination station {}.{RESET}", mission.destination);
                    return;
                }
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

    // Returns an Option containing a tuple: (Total Distance, Vector of Station Names in order)
    pub fn find_shortest_path(&self, origin: &str, destination: &str) -> Option<(f64, Vec<String>)> {
        
        // 1. The Scoreboard: Tracks the shortest known cumulative distance to each station
        let mut distances: HashMap<String, f64> = HashMap::new();
        
        // 2. The Breadcrumbs: Remembers the previous station so we can retrace our steps at the end
        let mut came_from: HashMap<String, String> = HashMap::new();
        
        // 3. The Queue: Our Min-Heap that always gives us the closest cumulative station
        let mut priority_queue = BinaryHeap::new();

        // Initialize: Set all known stations to Infinity
        for station in self.station_locations.keys() {
            distances.insert(station.clone(), f64::INFINITY);
        }

        // START THE WAVE: The distance from the origin to itself is 0.0
        distances.insert(origin.to_string(), 0.0);
        priority_queue.push(RouteState { cost: 0.0, station: origin.to_string() });

        // --- THE ALGORITHM LOOP GOES HERE ---
        // while let Some(RouteState { cost, station }) = priority_queue.pop() {
        //     ...
        // }

        // ... (Previous setup: HashMap initialization, pushing origin to queue)

        while let Some(RouteState { cost, station }) = priority_queue.pop() {
            
            // 1. THE STALE TICKET CHECK
            // If we pull a ticket that is worse than our current scoreboard, throw it away.
            let known_best = *distances.get(&station).unwrap_or(&f64::INFINITY);
            if cost > known_best {
                continue;
            }

            // 2. THE DESTINATION CHECK
            // If the station we just popped is our destination, we are done! The shortest path is locked.
            if station == destination {
                // Time to follow the breadcrumbs backwards!
                let mut path = Vec::new();
                let mut current = destination.to_string();
                
                while let Some(previous) = came_from.get(&current) {
                    path.push(current.clone());
                    current = previous.clone();
                }
                path.push(origin.to_string());
                path.reverse(); // Flip it so it goes Origin -> Destination
                
                return Some((cost, path));
            }

            // 3. THE SCOUTING PHASE
            // We are at a valid station. Let's look at all the tracks connected to it.
            for ((track_origin, track_dest), &track_distance) in &self.tracks {
                if track_origin == &station {
                    // Calculate the cumulative distance to this neighbor
                    let next_cost = cost + track_distance;
                    let neighbor_best = *distances.get(track_dest).unwrap_or(&f64::INFINITY);

                    // 4. THE DISCOVERY
                    // If this new path is strictly better than what the neighbor currently has...
                    if next_cost < neighbor_best {
                        // ...Update the scoreboard!
                        distances.insert(track_dest.clone(), next_cost);
                        // ...Leave a breadcrumb pointing back to how we got here!
                        came_from.insert(track_dest.clone(), station.clone());
                        // ...Print a new ticket and throw it in the queue!
                        priority_queue.push(RouteState { cost: next_cost, station: track_dest.clone() });
                    }
                }
            }
        }

        // If the queue empties and we never hit the `if station == destination` block, 
        // it means there is physically no track connecting them.
        //None

        None // Temporary return
    }

}