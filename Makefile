.PHONY: all run assets clean

# 默认执行 run
all: run

# 烘焙并打包所有资产 (Asset Cooker)
assets:
	@echo "=> 正在提取和烘焙 Minecraft 资产..."
	cargo run --bin minecrust-asset-cli -- pack --jar-path ./assets/raw/1.21.1.jar
	@echo "=> 资产处理完毕，输出到 ./assets/processed/assets.mca"

# 启动客户端渲染引擎 (需确保资产已生成)
run:
	@if [ ! -f "./assets/processed/assets.mca" ]; then \
		echo "=> 未检测到 assets.mca，自动执行资产打包..."; \
		make assets; \
	fi
	@echo "=> 启动 Minecrust 客户端..."
	cargo run --bin minecrust-client

# 清理构建缓存和生成的资产
clean:
	@echo "=> 清理 Cargo 缓存..."
	cargo clean
	@echo "=> 删除处理过的资产缓存..."
	rm -rf ./assets/processed/*

# 纯启动服务端 (Dedicated Server)
run-server:
	@echo "=> 启动 Minecrust 纯服务端..."
	cargo run --bin minecrust-server
