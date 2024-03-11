use std::{collections::BTreeMap, fs, io::Write, net::Ipv4Addr, path::Path};
use git2::{build::RepoBuilder, Repository};
use tempfile::{tempdir, TempDir};

const OPERATOR_IP_REPO_URL: &str = "https://github.com/gaoyifan/china-operator-ip.git";
const OPERATOR_IP_REPO_BRANCH: &str = "ip-lists";
const OPERATOR_IP_REPO_FILTER: [&str; 4] = ["cernet", "cmcc", "unicom", "chinanet"];
const IPLIST_REPO_URL: &str = "https://github.com/metowolf/iplist.git";

pub struct ReposDir {
    pub operator_ip_repo_dir: TempDir,
    pub iplist_repo_dir: TempDir
}

pub struct CIDRMap {
    data: BTreeMap<u32, CIDRDetail>
}

struct CIDRDetail {
    range: u8,
    label: String
}

pub struct IPv4QueryResult {
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
    pub fn read_operator_ip_repo(dir: &ReposDir) -> CIDRMap {
        let mut data: BTreeMap<u32, CIDRDetail> = BTreeMap::new();
        // 遍历目录
        for i in fs::read_dir(&dir.operator_ip_repo_dir).unwrap() {
            let i = i.unwrap();
            // 过滤文件
            if i.file_type().unwrap().is_dir() || i.file_name().to_str().unwrap().contains("6") || !i.file_name().to_str().unwrap().contains(".txt") || !OPERATOR_IP_REPO_FILTER.iter().any(|filter| i.file_name().to_str().unwrap().contains(filter)) {
                continue;
            }
            // 处理每行记录
            for j in fs::read_to_string(i.path()).unwrap().split("\n") {
                if j == "" {
                    continue;
                }
                let (ipv4, mask) = cidr_depacker(j);
                data.insert(ipv4, CIDRDetail{
                    range: mask, 
                    label: String::from(Path::new(i.file_name().as_os_str()).file_stem().unwrap().to_str().unwrap())
                });
            }
        };
        CIDRMap {
            data: data
        }
    }
    pub fn query_ipv4(&self, ipv4: u32) -> Option<IPv4QueryResult> {
        let left = self.data.range(..ipv4).next_back();
        let right = self.data.range(ipv4..).next();
        let check_range = |node: (&u32, &CIDRDetail)| {
            let (queryed_ipv4, queryed_detail) = node;
            if *queryed_ipv4 <= ipv4 && queryed_ipv4 + 2_u32.pow(u32::from(32 - queryed_detail.range)) > ipv4 {
                return Some(IPv4QueryResult { range: queryed_ipv4 + 2_u32.pow(u32::from(32 - queryed_detail.range)) - ipv4, label: queryed_detail.label.clone() });
            }
            None
        };
        left.and_then(check_range).or_else(||right.and_then(check_range))
    }
}

fn cidr_depacker(cidr: &str) -> (u32, u8) {
    let mut cidr = cidr.split("/");
    let ipv4 = cidr.next().unwrap();
    let mask: u8 = cidr.next().unwrap().parse().unwrap();
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

pub fn generate_csv(dir: &ReposDir, isp_database: &CIDRMap) {
    let mut csv = fs::File::create("ip-geolocation.csv").unwrap();
    let mut log = fs::File::create("ip-geolocation.log").unwrap();
    for i in fs::read_dir(dir.iplist_repo_dir.path().join("data/cncity/")).unwrap() {
        let i = i.unwrap();
        if i.file_type().unwrap().is_dir() {
            continue;
        }
        for j in fs::read_to_string(i.path()).unwrap().split("\n") {
            let (mut ipv4, mask) = cidr_depacker(j);
            let mut mask = 2_u32.pow(u32::from(32 - mask));
            loop {
                match isp_database.query_ipv4(ipv4) {
                    Some(the_result) => {
                        let start = Ipv4Addr::from(ipv4).to_string();
                        let end = Ipv4Addr::from(ipv4 + the_result.range - 1).to_string();
                        writeln!(csv, "{},{},{}-{}", start, end, Path::new(i.path().as_os_str()).file_stem().unwrap().to_str().unwrap(), the_result.label).unwrap();
                        if the_result.range < mask {
                            ipv4 += the_result.range;
                            mask -= the_result.range;
                        }
                        else {
                            break;
                        }
                    },
                    None => {
                        writeln!(log, "{}-{} is not converted completely!", Path::new(i.path().as_os_str()).file_stem().unwrap().to_str().unwrap(), j).unwrap();
                        break;
                    }
                }
                
            }
        }
    }
}