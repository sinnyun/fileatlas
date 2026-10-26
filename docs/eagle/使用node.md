# 使用 Node.js 原生 API

Eagle 插件支持使用 Node.js 的原生 API，因此我们可以享受到以下好处：

* 可以利用 Node.js 的强大功能来实现一些复杂的功能，比如数据处理、文件压缩、网络通信等。
* 可以使用现有的 Node.js 生态系统中的各种模块和库来快速实现应用程序的各种功能，避免重复造轮子。
* 可以构建跨平台的应用程序，因为 Node.js 在 Windows、macOS 都有很好的支持。

## 示例 <a href="#oedfw" id="oedfw"></a>

```javascript
const fs = require('fs');

// 讀取文件
fs.readFile('/path/to/file.txt', (err, data) => {
    if (err) throw err;
    console.log(data);
});

// 寫入文件
fs.writeFile('/path/to/file.txt', 'Hello, world!', (err) => {
    if (err) throw err;
    console.log('The file has been saved!');
});

```

这段代码会读取一个文件，然后在文件中写入一段文本。在读取和写入操作实现时，会在控制台中输出相应的信息。

除了 `fs` 模块外，Node.js 原生 API 还有许多实用的模块提供了一系列常用的功能。下面是一些常用的 Node.js 原生模块：

1. `http` 模块：提供了 HTTP 服务器和客户端的功能。
2. `path` 模块：提供了处理文件路径的工具函数。
3. `os` 模块：提供了获取操作系统信息的功能。
4. `crypto` 模块：提供了加密和解密的功能。
5. `zlib` 模块：提供了数据压缩和解压缩的功能。

## 推荐学习资源 <a href="#unfm7" id="unfm7"></a>

通过使用 Node.js 的原生 API，可以大大提升应用进程的灵活性和功能性。如果您是 Node.js 的新手，那么下面这些教程可能会对您有所帮助：

* MDN 的 Node.js 入门教程：<https://developer.mozilla.org/zh-CN/docs/Learn/Server-side/Express_Nodejs/Introduction>
* Traversy Media 的《Node.js Tutorial for Beginners》：<https://www.youtube.com/watch?v=TlB_eWDSMt4>
* freeCodeCamp 的《Node.js Basics Tutorial | Learn the Basics of Node.js in 30 Minutes》：<https://www.youtube.com/watch?v=w-7RQ46RgxU>
* The Net Ninja 的《Node.js Tutorial for Beginners》：<https://www.youtube.com/watch?v=w-7RQ46RgxU>

上面这些视频可以帮助您快速入门 Node.js 开发，了解 Node.js 的基础知识和实用技巧。
