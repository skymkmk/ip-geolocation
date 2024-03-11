use std::{collections::BTreeMap, fs::{self, DirEntry}, io::Write, net::Ipv4Addr, path::Path};
use git2::{build::RepoBuilder, Repository};
use tempfile::{tempdir, TempDir};

const OPERATOR_IP_REPO_URL: &str = "https://github.com/gaoyifan/china-operator-ip.git";
const OPERATOR_IP_REPO_BRANCH: &str = "ip-lists";
const OPERATOR_IP_REPO_FILTER: [&str; 4] = ["cernet", "cmcc", "unicom", "chinanet"];
const IPLIST_REPO_URL: &str = "https://github.com/metowolf/iplist.git";

pub struct ReposDir {
    operator_ip_repo_dir: TempDir,
    iplist_repo_dir: TempDir
}

struct CIDRMap {
    data: BTreeMap<u32, CIDRDetail>
}

struct CIDRDetail {
    range: u32,
    label: String
}

struct IPv4QueryResult {
    range: u32,
    label: String
}

impl ReposDir {
    pub fn fetch_data() -> ReposDir {
        let dir = ReposDir {
            operator_ip_repo_dir: tempdir().unwrap(),
            iplist_repo_dir: tempdir().unwrap()
        };
        RepoBuilder::new().branch(OPERATOR_IP_REPO_BRANCH).clone(OPERATOR_IP_REPO_URL, dir.operator_ip_repo_dir.as_ref()).unwrap();
        Repository::clone(IPLIST_REPO_URL, dir.iplist_repo_dir.as_ref()).unwrap();
        dir
    }
}

impl CIDRMap {
    fn new(dir: &Vec<DirEntry>) -> CIDRMap {
        let mut data: BTreeMap<u32, CIDRDetail> = BTreeMap::new();
        // 遍历目录
        for i in dir {
            let label = String::from(Path::new(i.file_name().as_os_str()).file_stem().unwrap().to_str().unwrap());
            // 处理每行记录
            for j in fs::read_to_string(i.path()).unwrap().split("\n") {
                if j == "" {
                    continue;
                }
                let (ipv4, mask) = cidr_depacker(j);
                fn insert_and_merge(ipv4: u32, mask: u32, label: &str, data: &mut BTreeMap<u32, CIDRDetail>) {
                    let mut start_ip = 0;
                    let mut range = 0;
                    let mut mutted = false;
                    let mut delete_flag = false;
                    if let Some((&queryed_ipv4, queryed_detail)) = data.range(..ipv4).next_back() {
                        if queryed_detail.label == *label && queryed_ipv4 + queryed_detail.range - 1 >= ipv4 && queryed_ipv4 + queryed_detail.range - 1 < ipv4 + mask - 1 {
                            start_ip = queryed_ipv4;
                            range = ipv4 + mask - queryed_ipv4;
                        }
                    }
                    if range != 0 {
                        data.get_mut(&start_ip).unwrap().range = range;
                        mutted = true;
                    }
                    else {
                        start_ip = ipv4;
                        range = mask;
                    }
                    if let Some((&queryed_ipv4, queryed_detail)) = data.range(start_ip..).next() {
                        if queryed_detail.label == *label && start_ip + mask - 1 >= queryed_ipv4 {
                            delete_flag = true;
                            if start_ip + mask < queryed_ipv4 + queryed_detail.range {
                                range = queryed_ipv4 + queryed_detail.range - start_ip;
                            }
                        }
                    }
                    if delete_flag {
                        data.get_mut(&start_ip).unwrap().range = range;
                        let next_key = *data.range(ipv4 + 1..).next().unwrap().0;
                        data.remove_entry(&next_key);
                        mutted = true;
                    }
                    if !mutted {
                        data.insert(ipv4, CIDRDetail{
                            range: mask, 
                            label: String::from(label)
                        });
                    }
                    else {
                        insert_and_merge(start_ip, range, label, data);
                    }
                }
                insert_and_merge(ipv4, mask, &label, &mut data);
            }
        };
        CIDRMap {
            data: data
        }
    }
    fn read_operator_ip_repo(dir: &ReposDir) -> CIDRMap {
        let mut dirs: Vec<DirEntry> = Vec::new();
        for i in fs::read_dir(&dir.operator_ip_repo_dir).unwrap() {
            let i = i.unwrap();
            if i.file_type().unwrap().is_dir() || i.file_name().to_str().unwrap().contains("6") || !i.file_name().to_str().unwrap().contains(".txt") || !OPERATOR_IP_REPO_FILTER.iter().any(|filter| i.file_name().to_str().unwrap().contains(filter)) {
                continue;
            }
            dirs.push(i);
        }
        CIDRMap::new(&dirs)
    }
    fn read_iplist_repo(dir: &ReposDir) -> CIDRMap {
        let mut dirs: Vec<DirEntry> = Vec::new();
        for i in fs::read_dir(dir.iplist_repo_dir.path().join("data/cncity/")).unwrap() {
            let i = i.unwrap();
            if i.file_type().unwrap().is_dir() {
                continue;
            }
            dirs.push(i);
        }
        CIDRMap::new(&dirs)
    }
    fn query_ipv4(&self, ipv4: u32) -> Option<IPv4QueryResult> {
        let left = self.data.range(..ipv4).next_back();
        let check_range = |node: (&u32, &CIDRDetail)| {
            let (queryed_ipv4, queryed_detail) = node;
            if *queryed_ipv4 <= ipv4 && queryed_ipv4 + queryed_detail.range > ipv4 {
                return Some(IPv4QueryResult { range: queryed_ipv4 + queryed_detail.range - ipv4, label: queryed_detail.label.clone() });
            }
            None
        };
        left.and_then(check_range)
    }
}

fn cidr_depacker(cidr: &str) -> (u32, u32) {
    let mut cidr = cidr.split("/");
    let ipv4 = cidr.next().unwrap();
    let mask: u8 = cidr.next().unwrap().parse().unwrap();
    let mask = 2_u32.pow(u32::from(32 - mask));
    let ipv4 = ipv4_to_int(ipv4);
    (ipv4, mask)
}

fn ipv4_to_int(ipv4: &str) -> u32{
    let mut between_dot_num = ipv4.split(".");
    let mut convert_between_dot_num = || {
        let digits: u8 = between_dot_num.next().unwrap().parse().unwrap();
        digits
    };
    u32::from(Ipv4Addr::new(convert_between_dot_num(), convert_between_dot_num(), convert_between_dot_num(), convert_between_dot_num()))
}

pub fn generate_csv(dir: &ReposDir) {
    let mut csv = fs::File::create("ip-geolocation.csv").unwrap();
    let mut log = fs::File::create("ip-geolocation.log").unwrap();
    let operator_ip_database = CIDRMap::read_operator_ip_repo(dir);
    let iplist_database = CIDRMap::read_iplist_repo(dir);
    for i in iplist_database.data {
        let (mut ipv4, ipv4_detail) = i;
        let mut mask = ipv4_detail.range;
        let iplist_label = ipv4_detail.label;
        loop {
            match operator_ip_database.query_ipv4(ipv4) {
                Some(the_result) => {
                    let start = Ipv4Addr::from(ipv4).to_string();
                    let end = Ipv4Addr::from(ipv4 + the_result.range.min(mask) - 1).to_string();
                    writeln!(csv, "{},{},{}-{}", start, end, iplist_label, the_result.label).unwrap();
                    if the_result.range < mask {
                        ipv4 += the_result.range;
                        mask -= the_result.range;
                    }
                    else {
                        break;
                    }
                },
                None => {
                    writeln!(log, "{}-{}/{} is not converted completely!", iplist_label, Ipv4Addr::from(ipv4).to_string(), 32 - f64::log2(f64::from(mask)) as u8).unwrap();
                    break;
                }
            }
            
        }
    }
}