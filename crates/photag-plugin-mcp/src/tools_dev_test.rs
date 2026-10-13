use super::*;

#[tool_router(router = dev_test_tool_router, vis = "pub(super)")]
impl super::PhoTagMcpServer {
    #[tool(
        description = "读取当前 PhoTag 开发测试插件状态和正在运行的任务。要求启动时启用 dev-test 配置。"
    )]
    fn dev_test_status(&self) -> String {
        let Some(service) = &self.dev_test else {
            return error_json(
                "开发测试插件未启用。请使用 PHOTAG_DEV_PLUGINS=dev-test 启动 MCP 子进程。",
            );
        };
        serde_json::to_string(&json!({
            "available": true,
            "running": service.is_running(),
            "status": service.status(),
        }))
        .unwrap_or_else(|error| error_json(&error.to_string()))
    }

    #[tool(
        description = "在当前隔离的 PhoTag 子进程中扫描 test-data/photag-library 的真实媒体文件。任务在后台运行；轮询 dev_test_status 获取进度和结果。"
    )]
    fn dev_test_scan_fixture(&self) -> String {
        let Some(service) = &self.dev_test else {
            return error_json("开发测试插件未启用；请用 dev-test 配置启动 MCP 子进程。");
        };
        let accepted = service.scan_fixture();
        serde_json::to_string(&json!({
            "accepted": accepted,
            "running": service.is_running(),
            "status": service.status(),
        }))
        .unwrap_or_else(|error| error_json(&error.to_string()))
    }

    #[tool(
        description = "在开发测试数据库中替换为 1,000、10,000 或 100,000 条虚拟图库记录，用于实际驱动图库负载。不会创建同量级实体文件；轮询 dev_test_status 获取进度和结果。"
    )]
    fn dev_test_inject_virtual(
        &self,
        Parameters(request): Parameters<DevTestInjectRequest>,
    ) -> String {
        let Some(service) = &self.dev_test else {
            return error_json("开发测试插件未启用；请用 dev-test 配置启动 MCP 子进程。");
        };
        let accepted = service.inject_virtual(request.count);
        serde_json::to_string(&json!({
            "accepted": accepted,
            "requested_count": request.count,
            "running": service.is_running(),
            "status": service.status(),
        }))
        .unwrap_or_else(|error| error_json(&error.to_string()))
    }

    #[tool(
        description = "清理当前开发测试库中保留给压力测试的虚拟媒体行，不删除样例目录中的真实文件。轮询 dev_test_status 获取完成状态。"
    )]
    fn dev_test_clear_virtual(&self) -> String {
        let Some(service) = &self.dev_test else {
            return error_json("开发测试插件未启用；请用 dev-test 配置启动 MCP 子进程。");
        };
        let accepted = service.clear_virtual();
        serde_json::to_string(&json!({
            "accepted": accepted,
            "running": service.is_running(),
            "status": service.status(),
        }))
        .unwrap_or_else(|error| error_json(&error.to_string()))
    }
}
