use ip_geolocation::{generate_csv, CIDRMap, ReposDir};


fn main() {
    println!("---Fetching data...---");
    let dirs = ReposDir::fetch_data();
    println!("-----Data fatched!----");
    println!("-----Compiling CSV----"); 
    let isp_database = CIDRMap::read_operator_ip_repo(&dirs);
    generate_csv(&dirs, &isp_database);
    println!("-----CSV compiled!----"); 
}
