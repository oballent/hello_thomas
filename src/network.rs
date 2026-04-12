use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;

// 1. The wrapper to hold a station and its cumulative distance in the queue
#[derive(Clone, PartialEq)]
struct RouteState {
    cost: f64,
    station: u32,
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



use crate::models::{Cargo, FreightOrder, Location, Mission, MissionReport, StationCommand, Train, TrainError};
use crate::facilities::Station;
//use std::collections::HashMap;
use std::sync::mpsc::{self, Sender};

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";




pub struct GlobalLedger {
    pub pending_cargo: Vec<FreightOrder>,
    pub active_missions: Vec<Mission>,
    pub next_mission_id: u32,
}

impl GlobalLedger {
    pub fn new() -> Self {
        GlobalLedger {
            pending_cargo: Vec::new(),
            active_missions: Vec::new(),
            next_mission_id: 1,
        }
    }
}
















pub struct RailwayNetwork {
    // Maps (Origin, Destination) -> Distance in km
    tracks: HashMap<u32, Vec<(u32, f64)>>,
    // The network also needs to hold the Stations themselves so it can route trains between them
    //stations: HashMap<String, Station>,
    // We no longer hold physical Stations. We hold the Transmitters to their Actor threads!
    station_handles: HashMap<u32, Sender<StationCommand>>,
    //missions: HashMap<u32, Mission>, // <-- The Source of Truth for all missions on the network
    station_locations: HashMap<u32, Location>
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
    


    pub fn register_station(&mut self, id: u32, location: Location, tx: Sender<StationCommand>) {
        self.station_locations.insert(id, location);
        self.station_handles.insert(id, tx);
    }


    pub fn add_track(&mut self, a: u32, b: u32) {
        // 1. Look up the locations from our internal directory
        let loc_a = self.station_locations.get(&a).expect("Station A not found!");
        let loc_b = self.station_locations.get(&b).expect("Station B not found!");

        // 2. Do the math internally (No manual work for the user!)
        let distance = loc_a.distance_to(loc_b);

        // 3. Insert both directions automatically
        self.tracks.entry(a).or_insert_with(Vec::new).push((b, distance));
        self.tracks.entry(b).or_insert_with(Vec::new).push((a, distance));

        println!("{CYAN}Network: Track laid between {} and {} ({:.2}km){RESET}", a, b, distance);
    }

    // pub fn add_mission(&mut self, mission: Mission) {
    //     println!("{YELLOW}Network Ledger: Registering Mission {}.{RESET}", mission.id);
    //     self.missions.insert(mission.id, mission);
    // }

    pub fn get_distance(&self, origin: u32, destination: u32) -> Option<f64> {
        // We create a temporary tuple of u32 objects to match the HashMap key signature.
        for v in self.tracks.get(&origin) {
            for (dest, dist) in v {
                if *dest == destination {//dereference the reference to compare the actual value
                    return Some(*dist);//dereference the reference to return the actual value
                }
            }
        };
        None
    }

    // pub fn get_mission(&self, mission_id: &u32) -> Option<&Mission> {
    //     self.missions.get(mission_id)
    // }


    pub fn get_station_handle(&self, station_id: u32) -> Option<&Sender<StationCommand>> {
        self.station_handles.get(&station_id)
    }

    pub fn dispatch_train_across_network(&self, mission: Mission) {


            // Get the shortest path and distance for this mission's origin and destination
            let (distance, route) = match self.find_shortest_path(mission.origin, mission.destination) {
                Some((d, r)) => {
                    println!(
                        "{YELLOW}Network: Shortest path for Mission {} is {} km via {:?}.{RESET}",
                        mission.id, d, r
                    );
                    (d, r)
                },
                None => {
                    println!("{RED}Network Error: No track laid between {} and {}.{RESET}", mission.origin, mission.destination);
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
            let (transit_tx, transit_rx) = mpsc::channel::<Result<Train, TrainError>>();
            
            
            // 5. Send the command to the Origin!
            println!("{YELLOW}Network: Ordering {} to assemble Mission {}.{RESET}", mission.origin, mission.id);
            let _ = origin_tx.send(StationCommand::AssembleMission {
                mission: mission.clone(),
                distance,
                reply_to: transit_tx,
                route: route.clone(),
                destination: mission.destination.clone(),
            });
    }

    // Returns an Option containing a tuple: (Total Distance, Vector of Station Names in order)
    pub fn find_shortest_path(&self, origin: u32, destination: u32) -> Option<(f64, Vec<u32>)> {
        
        // 1. The Scoreboard: Tracks the shortest known cumulative distance to each station
        let mut distances: HashMap<u32, f64> = HashMap::new();
        
        // 2. The Breadcrumbs: Remembers the previous station so we can retrace our steps at the end
        let mut came_from: HashMap<u32, u32> = HashMap::new();
        
        // 3. The Queue: Our Min-Heap that always gives us the closest cumulative station
        let mut priority_queue = BinaryHeap::new();

        // Initialize: Set all known stations to Infinity
        for station in self.station_locations.keys() {
            distances.insert(*station, f64::INFINITY);
        }

        // START THE WAVE: The distance from the origin to itself is 0.0
        distances.insert(origin, 0.0);
        priority_queue.push(RouteState { cost: 0.0, station: origin });

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
                continue;// Copilot was here. He said this is the key optimization that keeps Dijkstra's algorithm efficient. Without this check, we would process every single path to every station, even if we already found a better one. With this check, we only process each station once with its best known distance, and ignore all the "stale" tickets that are worse than what we already have on the scoreboard. Thanks, Copilot!
            }

            // 2. THE DESTINATION CHECK
            // If the station we just popped is our destination, we are done! The shortest path is locked.
            if station == destination {
                // Time to follow the breadcrumbs backwards!
                let mut path = Vec::new();
                let mut current = destination;
                
                while let Some(previous) = came_from.get(&current) {
                    path.push(current.clone());
                    current = previous.clone();
                }
                path.push(origin);
                path.reverse(); // Flip it so it goes Origin -> Destination
                
                return Some((cost, path));
            }

            // 3. THE SCOUTING PHASE
            // We are at a valid station. Let's look at all the tracks connected to it.
            
            if let Some(v) = self.tracks.get(&station) {
                for (track_dest, track_distance) in v {
                    // Calculate the cumulative distance to this neighbor
                    let next_cost = cost + track_distance;
                    let neighbor_best = *distances.get(track_dest).unwrap_or(&f64::INFINITY);

                    // 4. THE DISCOVERY
                    // If this new path is strictly better than what the neighbor currently has...
                    if next_cost < neighbor_best {
                        // ...Update the scoreboard!
                        distances.insert(*track_dest, next_cost);
                        // ...Leave a breadcrumb pointing back to how we got here!
                        came_from.insert(*track_dest, station);
                        // ...Print a new ticket and throw it in the queue!
                        priority_queue.push(RouteState { cost: next_cost, station: *track_dest });
                    }
                }
            };
        }

        // If the queue empties and we never hit the `if station == destination` block, 
        // it means there is physically no track connecting them.
        //None

        None // Temporary return
    }

}