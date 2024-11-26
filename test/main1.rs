use std::{ error::Error, fs::File, io::{ self, Write }, path::PathBuf, str };
use csv::Writer;
use reqwest::Client;
use serde::Deserialize;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let folder_path = "api.bgpview.io";
    match create_folder_if_not_exists(folder_path) {
        Ok(_path) => {}
        Err(e) => eprintln!("Error creating folder: {}", e),
    }
    let version = get_prefixes_version();
    let asn = get_user_input();

    let url = format!("https://api.bgpview.io/asn/{}/prefixes", asn);

    let client = Client::builder().user_agent("Mozilla/5.0").build()?;
    let response = client.get(url).send().await?;

    let output_csv = format!("{}/AS{}_v{}.csv", folder_path, asn, version);
    let output_txt = format!("{}/AS{}_v{}.txt", folder_path, asn, version);

    if response.status().is_success() {
        let json: ApiResponse = response.json().await?;
        if json.status == "ok" {
            // 写入csv文件
            let mut wtr = Writer::from_writer(File::create(output_csv)?);
            wtr.write_record(&["IP地址前缀", "国家代码", "名称", "描述", "rir名称"])?;
            // 写入txt文件
            let mut file = File::create(output_txt).expect("Failed to create txt file");
            // 处理数据
            match version {
                "4" => {
                    json.data.ipv4_prefixes.iter().for_each(|p| {
                        println!(
                            "抓取到内容：{:?}",
                            vec![
                                p.prefix.clone(),
                                p.country_code.clone().unwrap_or_default(),
                                p.description.clone().unwrap_or_default()
                            ]
                        );
                        // 写入csv
                        wtr.write_record(
                            &[
                                p.prefix.clone(),
                                p.country_code.clone().unwrap_or_default(),
                                p.name.clone().unwrap_or_default(),
                                p.description.clone().unwrap_or_default(),
                                p.parent.rir_name.clone().unwrap_or_default(),
                            ]
                        ).expect("Failed to write to csv file");
                        // 写入txt
                        writeln!(file, "{}", p.prefix.clone()).expect(
                            "Failed to write to txt file"
                        );
                    });
                }
                "6" => {
                    json.data.ipv6_prefixes.iter().for_each(|p| {
                        println!(
                            "抓取到内容：{:?}",
                            vec![
                                p.prefix.clone(),
                                p.country_code.clone().unwrap_or_default(),
                                p.description.clone().unwrap_or_default()
                            ]
                        );
                        // 写入csv
                        wtr.write_record(
                            &[
                                p.prefix.clone(),
                                p.country_code.clone().unwrap_or_default(),
                                p.name.clone().unwrap_or_default(),
                                p.description.clone().unwrap_or_default(),
                                p.parent.rir_name.clone().unwrap_or_default(),
                            ]
                        ).expect("Failed to write to csv file");
                        // 写入txt
                        writeln!(file, "{}", p.prefix.clone()).expect(
                            "Failed to write to txt file"
                        );
                    });
                }
                _ => unreachable!(),
            }
        } else {
            eprintln!("获取到的数据状态不是ok，而是{}", json.status);
        }
    } else {
        eprintln!("获取API数据失败: {}", response.status());
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    status: String,
    data: Data,
}

#[derive(Debug, Deserialize)]
struct Data {
    ipv4_prefixes: Vec<Prefix>,
    ipv6_prefixes: Vec<Prefix>,
}

#[derive(Debug, Deserialize)]
struct Prefix {
    prefix: String,
    name: Option<String>,
    country_code: Option<String>,
    description: Option<String>,
    parent: Parent,
}

#[derive(Debug, Deserialize)]
struct Parent {
    rir_name: Option<String>,
}

// 文件夹不存在就创建
fn create_folder_if_not_exists(folder_path: &str) -> Result<PathBuf, std::io::Error> {
    let folder_path = PathBuf::from(folder_path);

    if folder_path.exists() {
        Ok(folder_path)
    } else {
        std::fs::create_dir_all(&folder_path)?;
        Ok(folder_path)
    }
}

fn get_prefixes_version() -> &'static str {
    let prefix_shape =
        r"
  +------------------+
  |                  |
  |  1. IPv4 Prefix  |
  |  2. IPv6 Prefix  |
  |                  |
  +------------------+
";
    println!("{}", prefix_shape);
    loop {
        print!("选择对应的数字:");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        match input {
            "1" => {
                return "4";
            }
            "2" => {
                return "6";
            }
            _ => {}
        }
    }
}

fn get_user_input() -> String {
    loop {
        print!("请输入要下载的AS(可以带有'AS'前缀):");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_uppercase();

        if let Some(num) = input.strip_prefix("AS").and_then(|s| s.parse::<usize>().ok()) {
            if (1..=999999).contains(&num) {
                return format!("{}", num);
            }
        } else if let Ok(num) = input.parse::<usize>() {
            if (1..=999999).contains(&num) {
                return format!("{}", num);
            }
        }
    }
}
