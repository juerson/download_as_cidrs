mod models;

use crate::models::ApiResponse; // 该结构体只用于api.bgpview.io
use std::{ error::Error, fs::File, io::Write, path::PathBuf, str };
use ipnetwork::IpNetwork;
use csv::Writer;
use regex::Regex;
use reqwest::{ header::USER_AGENT, Client };
use clap::{ error::ErrorKind, CommandFactory, Parser };
use select::{ document::Document, predicate::{ Attr, Name, Predicate } };

/// 本工具用于下载自治系统ASN的CIDR，有3个API源，分别对应bgpview.io、bgp.he.net、bgp.tools。
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// 指定(自治系统)ASN，输入数字，不包含AS
    #[arg(long = "as")]
    asn: u32,

    /// 指定CIDR的版本，输入4或6
    #[arg(short, long, default_value_t = 4)]
    cidr_version: u8,

    /// 使用哪个API URL源下载，0为"bgpview.io"，1为"bgp.he.net", 2为"bgp.tools"
    #[arg(short = 'i', default_value_t = 0)]
    api_url_index: u8,
}

static API_URL: &[&str] = &["api.bgpview.io", "bgp.he.net", "bgp.tools"];
static CLIENT_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

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

// 该函数应用到"bgp.he.net"中
fn get_country_code_from_gifurl(url: &str) -> Option<&str> {
    let re = Regex::new(r"([^/]+)\.").unwrap();

    re.captures(url)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let result = Args::try_parse();
    match result {
        Ok(args) => {
            let save_folder_path = API_URL[args.api_url_index as usize];
            // 检查要保存到的文件夹是否存在，不存在则创建
            match create_folder_if_not_exists(save_folder_path) {
                Ok(_path) => {}
                Err(e) => eprintln!("Error creating folder: {}", e),
            }

            // 输出的csv文件和txt文件
            let output_csv = format!(
                "{}/AS{}_v{}.csv",
                save_folder_path,
                args.asn,
                args.cidr_version
            );
            let output_txt = format!(
                "{}/AS{}_v{}.txt",
                save_folder_path,
                args.asn,
                args.cidr_version
            );

            // 按照不同的API_URL来源，下载asn的cidr
            match args.api_url_index {
                0 => {
                    let client = Client::builder().user_agent("Mozilla/5.0").build()?;
                    let response = client
                        .get(format!("https://{}/asn/{}/prefixes", API_URL[0], args.asn))
                        .send().await?;
                    download_api_bgpview_io(response, &args, &output_csv, &output_txt).await?;
                }
                1 => {
                    let client = reqwest::Client::new();
                    let response = client
                        .get(
                            format!(
                                "https://{}/AS{}#_prefixes{}",
                                API_URL[1],
                                args.asn,
                                args.cidr_version
                            )
                                .trim_end_matches('4') // 如果后面的数字是4，则去掉
                                .to_string()
                        )
                        .header(USER_AGENT, CLIENT_USER_AGENT)
                        .send().await?;
                    download_bgp_he_net(response, &args, &output_csv, &output_txt).await?;
                }
                2 => {
                    let client = reqwest::Client::new();
                    let response = client
                        .get(format!("https://{}/as/{}#prefixes", API_URL[2], args.asn))
                        .header(USER_AGENT, CLIENT_USER_AGENT)
                        .send().await?;
                    download_bgp_tools(response, &args, &output_csv, &output_txt).await?;
                }
                _ => panic!("Invalid api_url_index"),
            };
        }
        Err(e) => {
            if
                e.kind() == ErrorKind::MissingRequiredArgument ||
                e.kind() == ErrorKind::InvalidValue
            {
                // 如果是因为缺少必需参数或无效值导致的错误，则显示帮助信息
                Args::command().print_help().unwrap();
            } else {
                // 其他类型的错误则正常打印错误信息
                e.print().unwrap();
            }
        }
    }

    Ok(())
}

async fn download_api_bgpview_io(
    response: reqwest::Response,
    args: &Args,
    output_csv: &str,
    output_txt: &str
) -> Result<(), Box<dyn Error>> {
    if response.status().is_success() {
        let json: ApiResponse = response.json().await?;
        if json.status == "ok" {
            // 创建一个csv文件
            let mut wtr = Writer::from_writer(File::create(output_csv)?);
            wtr.write_record(&["IP地址前缀", "国家代码", "名称", "描述", "rir名称"])?;
            // 创建一个txt文件
            let mut file = File::create(output_txt).expect("Failed to create txt file");
            // 处理数据
            match args.cidr_version {
                4 => {
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
                                p.prefix.to_string(),
                                p.country_code.clone().unwrap_or_default(),
                                p.name.clone().unwrap_or_default(),
                                p.description.clone().unwrap_or_default(),
                                p.parent.rir_name.clone().unwrap_or_default(),
                            ]
                        ).expect("Failed to write to csv file");
                        // 写入txt
                        writeln!(file, "{}", p.prefix.to_string()).expect(
                            "Failed to write to txt file"
                        );
                    });
                }
                6 => {
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
                                p.prefix.to_string(),
                                p.country_code.clone().unwrap_or_default(),
                                p.name.clone().unwrap_or_default(),
                                p.description.clone().unwrap_or_default(),
                                p.parent.rir_name.clone().unwrap_or_default(),
                            ]
                        ).expect("Failed to write to csv file");
                        // 写入txt
                        writeln!(file, "{}", p.prefix.to_string()).expect(
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
        eprintln!("HTTP网页请求失败，状态码是: {}", response.status());
    }
    Ok(())
}

async fn download_bgp_he_net(
    response: reqwest::Response,
    args: &Args,
    output_csv: &str,
    output_txt: &str
) -> Result<(), Box<dyn Error>> {
    if response.status().is_success() {
        // 创建一个csv文件
        let mut writer = Writer::from_path(output_csv).unwrap();
        writer.write_record(vec!["IP地址前缀", "国家代码", "国家名称", "描述"]).unwrap();
        // 创建一个txt文件
        let mut file = File::create(output_txt).expect("Failed to create file");

        // 获取 HTML 内容为字符串
        let content = response.text().await?;
        // 使用 select 解析 HTML
        let document = Document::from(content.as_str());

        // 匹配对应的表格ID
        let table_id = match args.cidr_version {
            4 => "table_prefixes4",
            6 => "table_prefixes6",
            _ => panic!("Invalid version"),
        };

        println!();

        // 找到表格的所有行
        for row in document.find(Attr("id", table_id).descendant(Name("tr"))) {
            let cells: Vec<_> = row
                .find(Name("td"))
                .map(|cell: select::node::Node<'_>| {
                    // 创建一个向量，用于存储结果
                    let mut elements = Vec::new();

                    // 查找 div.flag 下的 img 元素
                    if
                        let Some(div) = cell
                            .find(Attr("class", "flag alignright floatright"))
                            .next()
                    {
                        for img in div.find(Name("img")) {
                            if let Some(src) = img.attr("src") {
                                let substring = get_country_code_from_gifurl(src);
                                match substring {
                                    Some(s) => elements.push(s.to_uppercase()), // country code
                                    None => elements.push("".to_string()),
                                }
                            }
                            if let Some(title) = img.attr("title") {
                                elements.push(title.to_string()); // country name
                            } else {
                                elements.push("".to_string());
                            }
                        }
                    }

                    // 添加 td 的文本内容
                    let text = cell.text().trim().to_string();
                    if !text.is_empty() {
                        elements.push(text);
                    } else {
                        elements.push("".to_string());
                    }

                    elements // 返回当前单元格解析出的内容
                })
                .collect();
            if !cells.is_empty() {
                let one_dimensional: Vec<String> = cells.clone().into_iter().flatten().collect(); // 将二维向量转换为一维向量
                // 判断CIDR的类型，4 或 6？
                if let Ok(ip_network) = one_dimensional[0].parse::<IpNetwork>() {
                    match ip_network {
                        IpNetwork::V4(v4_network) => {
                            if args.cidr_version == 4 {
                                println!("抓取到内容：{:?}", one_dimensional);
                                writer
                                    .write_record(&one_dimensional)
                                    .expect("Failed to write to csv file");
                                writeln!(file, "{}", v4_network.to_string()).expect(
                                    "Failed to write to txt file"
                                );
                            }
                        }
                        IpNetwork::V6(v6_network) => {
                            if args.cidr_version == 6 {
                                println!("抓取到内容：{:?}", one_dimensional);
                                writer
                                    .write_record(&one_dimensional)
                                    .expect("Failed to write to csv file");
                                writeln!(file, "{}", v6_network.to_string()).expect(
                                    "Failed to write to txt file"
                                );
                            }
                        }
                    }
                }
            }
        }
    } else {
        println!("HTTP网页请求失败，状态码是: {}", response.status());
    }
    Ok(())
}

async fn download_bgp_tools(
    response: reqwest::Response,
    args: &Args,
    output_csv: &str,
    output_txt: &str
) -> Result<(), Box<dyn Error>> {
    if response.status().is_success() {
        // 创建一个csv文件
        let mut writer = Writer::from_path(output_csv).unwrap();
        writer.write_record(vec!["IP地址前缀", "国家代码", "描述"]).unwrap();
        // 创建一个txt文件
        let mut file = File::create(output_txt).expect("Failed to create file");

        // 获取 HTML 内容为字符串
        let content = response.text().await?;
        // 使用 select 解析 HTML
        let document = Document::from(content.as_str());

        println!();

        // 找到表格的所有行
        for row in document.find(
            Attr("id", "donotscrapebgptools-prefixlist-tbody").descendant(Name("tr"))
        ) {
            let cells: Vec<_> = row
                .find(Name("td"))
                .map(|cell: select::node::Node<'_>| {
                    // 创建一个向量，用于存储结果
                    let mut elements = Vec::new();

                    // 查找 img 元素的国家代码
                    if let Some(img) = cell.find(Name("img")).next() {
                        // 获取第一个 img
                        if let Some(title) = img.attr("title") {
                            // 获取第一个 img 的 title
                            elements.push(title.to_string());
                        }
                    }

                    // 添加 td 的文本内容
                    let text = cell.text().trim().to_string();
                    if !text.is_empty() {
                        elements.push(text);
                    } else {
                        elements.push("".to_string()); // 添加空字符串，占位
                    }

                    elements // 返回当前单元格解析出的内容
                })
                .collect();
            if !cells.is_empty() {
                let one_dimensional: Vec<String> = cells.into_iter().flatten().collect(); // 将二维向量转换为一维向量
                // 调整元素排列顺序，以及过滤掉不要的元素
                let transformed_vec = vec![
                    one_dimensional
                        .get(2)
                        .cloned()
                        .unwrap_or_else(|| "".to_string()), // 第3个元素
                    one_dimensional
                        .get(0)
                        .cloned()
                        .unwrap_or_else(|| "".to_string()), // 第1个元素
                    one_dimensional
                        .get(3)
                        .cloned()
                        .unwrap_or_else(|| "".to_string()) // 第4个元素
                ];
                // 判断CIDR的类型，4 或 6？
                if let Ok(ip_network) = transformed_vec[0].parse::<IpNetwork>() {
                    match ip_network {
                        IpNetwork::V4(v4_network) => {
                            if args.cidr_version == 4 {
                                println!("抓取到内容：{:?}", transformed_vec);
                                writer
                                    .write_record(&transformed_vec)
                                    .expect("Failed to write to csv file");
                                writeln!(file, "{}", v4_network.to_string()).expect(
                                    "Failed to write to txt file"
                                );
                            }
                        }
                        IpNetwork::V6(v6_network) => {
                            if args.cidr_version == 6 {
                                println!("抓取到内容：{:?}", transformed_vec);
                                writer
                                    .write_record(&transformed_vec)
                                    .expect("Failed to write to csv file");
                                writeln!(file, "{}", v6_network.to_string()).expect(
                                    "Failed to write to txt file"
                                );
                            }
                        }
                    }
                }
            }
        }
    } else {
        println!("HTTP网页请求失败，状态码是: {}", response.status());
    }
    Ok(())
}
