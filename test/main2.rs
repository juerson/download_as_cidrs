use regex::Regex;
use reqwest::header::USER_AGENT;
use std::{ error::Error, fs::File, io::{ self, Write }, path::PathBuf };
use select::{ document::Document, predicate::{ Attr, Name, Predicate } };
use csv::Writer;

fn create_folder_if_not_exists(folder_path: &str) -> Result<PathBuf, std::io::Error> {
    let folder_path = PathBuf::from(folder_path);

    if folder_path.exists() {
        Ok(folder_path)
    } else {
        std::fs::create_dir_all(&folder_path)?;
        Ok(folder_path)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let folder_path = "bgp.he.net";
    match create_folder_if_not_exists(folder_path) {
        Ok(_path) => {}
        Err(e) => eprintln!("Error creating folder: {}", e),
    }
    let version = get_prefixes_version();
    let asn = get_user_input();

    let file_name = match version {
        "" => format!("{}/{}_v4", folder_path, asn),
        "6" => format!("{}/{}_v6", folder_path, asn),
        _ => panic!("Invalid version"),
    };

    let url = format!("https://bgp.he.net/{asn}#_prefixes{version}");

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header(
            USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
        )
        .send().await?;

    if response.status().is_success() {
        let mut writer = Writer::from_path(format!("{file_name}.csv")).unwrap();
        writer.write_record(vec!["IP地址前缀", "国家代码", "国家名称", "描述"]).unwrap();

        let mut file = File::create(format!("{file_name}.txt")).expect("Failed to create file");

        // 获取 HTML 内容为字符串
        let content = response.text().await?;

        // 使用 select 解析 HTML
        let document = Document::from(content.as_str());

        let table_id = match version {
            "" => "table_prefixes4",
            "6" => "table_prefixes6",
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
                println!("{:?}", one_dimensional); // 输出表格内容
                writer.write_record(&one_dimensional).unwrap();
                writeln!(file, "{}", one_dimensional[0]).unwrap();
            }
        }
    } else {
        println!("Failed to fetch the page. Status: {}", response.status());
    }

    Ok(())
}

fn get_country_code_from_gifurl(url: &str) -> Option<&str> {
    let re = Regex::new(r"([^/]+)\.").unwrap();
    re.captures(url)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
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
                return input;
            }
        } else if let Ok(num) = input.parse::<usize>() {
            if (1..=999999).contains(&num) {
                return format!("AS{}", input);
            }
        }
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
                return "";
            }
            "2" => {
                return "6";
            }
            _ => {}
        }
    }
}
