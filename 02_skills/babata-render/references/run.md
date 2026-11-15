# PNG 运行入口

依赖 Node、Playwright Chromium、marked。可在 05_scripts/atoms 安装锁定依赖；已有依赖则用 BABATA_NODE_MODULES 指定 node_modules。BABATA_CHROMIUM_EXECUTABLE 可指定兼容浏览器。不在 Git 放依赖目录、浏览器或凭据。

    node 05_scripts/atoms/render.mjs render --input <absolute.md-or.html> --out <absolute-new-directory> --mode long --width 1120
    node 05_scripts/atoms/render.mjs verify --out <absolute-directory>

支持 Markdown／HTML。资产限定输入目录内图片、CSS、字体，不跟随 symlink 越界；不支持远程资源、脚本、frame、播放器。SVG 等未接入类型须显式转换并记录限制。long 最高 30000px，超限失败；paged 要求全部内容包在直接子级 .page 容器中，一容器一完整页，不自动切表格。

输出 babata.png-render/v1 JSON，含输入／资产／HTML 快照及 hash、PNG hash、浏览器、参数、机械检查和未做视觉检查。manifest 最后写入，没有成功 manifest 不算成功。verify 只读冻结 bundle，无需浏览器或原路径；同 run render 还核当前输入／资产。环境升级需新 run，不承诺跨环境像素字节相同。

正式输出本地配置 BABATA_PNG_RENDERER 为 render.mjs 绝对路径，BABATA_NODE 可指定 Node。Rust 核验产物后写原 C2 manifest；未配置时不列为可构建格式。
依赖 probe 检查 Node 包和浏览器文件，实际启动仍以 build 结果为准。正式 renderer 超时会请求终止该子进程树；不能据此宣称父宿主被强制结束时也有 OS 级任务容器保证。
冻结 bundle 可独立 verify；资产快照用于完整性核查，不承诺直接打开 document.html 就按原相对目录重建。

    babata --json outputs build --kind png --record <record-id> --description <description>
    babata --json outputs verify --output <output-id>
    babata --json outputs rebuild --output <output-id>

verify 比较 hash 和当前权威输入，不重新截图做字节对比。正式 build 每次产生新 output，不是幂等键；未知结果先用 output ID 查询／核查，不能盲重投。
