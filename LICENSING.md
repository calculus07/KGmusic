# 许可说明（Licensing）

本仓库由**两个相互独立的程序**聚合而成，它们各自适用不同的许可证，不存在覆盖全仓库的单一许可证。

| 目录 | 内容 | 许可证 | 许可证全文 |
| --- | --- | --- | --- |
| `server/` | 酷狗 API 服务端（Rust） | **AGPL-3.0** | [`server/LICENSE`](server/LICENSE) |
| `web/` | 前端（Vue 3 + Vite） | **GPL-2.0-only** | [`web/LICENSE`](web/LICENSE) |

两者只通过 HTTP 通信，不互相链接、不编译到一起，因此可以按「聚合分发」放在同一个仓库里，
各自适用各自的许可证。**使用、修改或分发任一部分时，请遵守该部分对应的许可证。**

---

## 一、`server/` —— GNU Affero General Public License v3.0

`server/` 移植自 [md3Music](https://github.com/zzyoxml/md3Music) 的 `kugou_api_server`
（该项目采用 AGPL-3.0）。作为派生作品，本目录必须沿用 AGPL-3.0。

作者新增的部分（`/team/*` 五个接口、`/sidedt`、静态文件托管、命令行入口）也编译在同一个
crate 内，因此一并按 AGPL-3.0 分发。

> ### ⚠️ AGPL-3.0 第 13 条（网络条款）
>
> 如果修改本程序并让它**通过网络对外提供服务**，你**必须向使用者提供对应源码**。
>
> KGmusic 正是以此为部署形态，所以这不是可选项。将本服务上线时，请在页面显眼处
> （页脚、「关于」页等）提供一个指向你所部署版本源码的链接。

## 二、`web/` —— GNU General Public License v2.0 only

`web/` 来自 [MoeKoeMusic](https://github.com/iAJue/MoeKoeMusic)（MoeKoe 萌音前端）。

上游 `package.json` 声明的是 `"license": "GPL-2.0-only"`，即**仅有第 2 版**。
AGPL/上游 LICENSE 全文里出现的 `or (at your option) any later version` 属于 FSF 的附录
模板（GPLv2 第 282 行起的 *How to Apply These Terms*），上游从未把它填进自己的任何文件，
因此不能据此推定为 "or later"。**本目录只能按 GPL-2.0 分发，不能升级到 GPLv3 或 AGPL。**

## 三、为什么不能统一成一个许可证

AGPL-3.0 与 GPL-2.0-only 是自由软件基金会明确列出的**不兼容**组合：

* AGPL-3.0 第 13 条允许与 **GPLv3** 组合；
* 但 GPL-2.0-**only** 无法被升级到 v3，所以无法与 AGPL-3.0 合并且分发。

因此本仓库只能分别标注。若将来 MoeKoeMusic 上游把许可改为 `GPL-2.0-or-later`，
`web/` 便可按 GPL-3.0 取用，整个 KGmusic 即可统一为 AGPL-3.0。

## 四、上游署名

| 项目 | 许可证 | 在本仓库中的角色 |
| --- | --- | --- |
| [md3Music](https://github.com/zzyoxml/md3Music) | AGPL-3.0 | `server/` 的 Rust 实现移植自其 `kugou_api_server` |
| [MoeKoeMusic](https://github.com/iAJue/MoeKoeMusic) | GPL-2.0-only | `web/` 前端，以及 `server/` 中接口行为的对照实现 |
| [KuGouMusicApi](https://github.com/MakcRe/KuGouMusicApi) | MIT | `/team/*` 与 `/sidedt` 的行为参照（MIT 与 AGPL 兼容） |

## 五、第三方组件

| 组件 | 位置 | 许可证 |
| --- | --- | --- |
| tiny_http | `server/vendor/tiny_http/` | Apache-2.0 或 MIT 双许可，见该目录下 `LICENSE-APACHE` / `LICENSE-MIT` |
| Rust 依赖 | `server/Cargo.toml` 的 `[dependencies]` | 各自的许可证，见 crates.io 对应页面及各 crate 源码 |
| npm 依赖 | `web/node_modules/` | 各自的许可证，随包分发于各包的 `LICENSE` 文件 |

## 六、关于「禁止商用」

上游 MoeKoeMusic 的 README 中另有「本项目仅供个人学习研究使用，禁止用于商业及非法用途」一句。

需要说明：这是一条**附加限制**，而 GPL-2.0 第 6 条明确禁止在分发时增加额外限制，
故该句作为许可条款不产生效力，实际以各许可证原文为准。此处保留它，
是因为它反映了上游作者的意愿，供你在使用前自行斟酌。

---

> 本文档基于三个上游仓库中的 LICENSE 文件与 SPDX 标识作出的技术性说明，**不构成法律意见**。
> 涉及商业使用或大规模分发时，请咨询专业律师。
