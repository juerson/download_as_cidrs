# download_as_cidrs
本代码库的作用：根据asn的数字(自治系统)，去抓取/下载 bgpview.io、bgp.he.net、bgp.tools 网站的CIDR数据（区分v4或v6版本）

### 操作对象

 - HTML的元素属性和元素内容 (bgp.he.net、bgp.tools)
 - JSON数据 (api.bgpview.io)

### 关键技术

- clap (CLI命令行构建工具)
- tokio、reqwest (异步网络请求库)
- select (分析HTML的属性和元素内容)