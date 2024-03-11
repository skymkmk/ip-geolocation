use ip_geolocation::{generate_csv, ReposDir};


fn main() {
    println!("---Fetching data...---");
    let dirs = ReposDir::fetch_data();
    println!("-----Data fatched!----");
    println!("-----Compiling CSV----"); 
    generate_csv(&dirs);
    println!("-----CSV compiled!----"); 
}
