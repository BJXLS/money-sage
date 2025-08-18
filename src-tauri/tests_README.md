# Money Sage 测试说明文档

## 📝 测试概览

本项目为 `process_quick_booking_text` 方法创建了全面的测试套件，确保快速记账功能的可靠性。

## 🧪 测试结构

### 测试模块组织
```
src-tauri/src/
├── tests/
│   ├── mod.rs                     # 测试模块入口，包含通用测试工具
│   └── quick_booking_tests.rs     # 快速记账功能专项测试
└── lib.rs                         # 主模块，包含测试模块声明
```

### 测试工具函数

#### `create_test_database()` 
- 创建内存SQLite数据库
- 自动插入测试分类数据（餐饮、交通、工资等）
- 插入测试LLM配置

#### `create_test_database_state()`
- 创建DatabaseState实例供测试使用

## 🔍 测试用例详情

### 1. 空输入测试 (`test_process_quick_booking_text_empty_input`)
- **目的**: 验证空字符串输入的处理
- **预期**: 返回失败，消息为"输入文本不能为空"

### 2. 无LLM配置测试 (`test_process_quick_booking_text_no_llm_config`)
- **目的**: 测试未配置AI平台时的处理
- **预期**: 返回失败，提示配置大模型平台和API密钥

### 3. 单条记录成功测试 (`test_process_quick_booking_text_success_single_transaction`)
- **目的**: 测试成功解析单条记账记录
- **输入**: `"今天中午花了25.5元吃午餐"`
- **预期**: 成功创建1条支出记录，金额25.5，分类为餐饮

### 4. 多条记录测试 (`test_process_quick_booking_text_multiple_transactions`)
- **目的**: 测试解析多条记账记录
- **输入**: `"今天中午花了28.5元吃午餐，晚上打车回家15元"`
- **预期**: 成功创建2条记录（餐饮28.5元 + 交通15元）

### 5. 未知分类测试 (`test_process_quick_booking_text_unknown_category`)
- **目的**: 测试AI返回未知分类时的处理
- **预期**: 回退到默认分类"其他支出"或报告分类错误

### 6. 无效日期测试 (`test_process_quick_booking_text_invalid_date`)
- **目的**: 测试AI返回无效日期格式的处理
- **输入**: 日期为 `"invalid-date"`
- **预期**: 记录处理失败，错误信息包含"日期解析失败"

### 7. 收入记录测试 (`test_process_quick_booking_text_income_transaction`)
- **目的**: 测试收入类型记录的处理
- **输入**: `"今天收到了5000元工资"`
- **预期**: 成功创建收入记录，分类为工资

## 🛠️ 测试实现特点

### 模拟AI响应
- 由于实际AI调用需要网络和API密钥，测试使用模拟AI响应
- `test_process_quick_booking_with_mock_ai_response()` 函数跳过AI调用，直接使用预定义JSON响应
- 保持与实际处理流程完全一致的数据验证和数据库操作

### 数据隔离
- 每个测试使用独立的内存数据库
- 测试之间无数据污染
- 支持并发测试执行

### 全面覆盖
- ✅ 输入验证（空输入）
- ✅ 配置验证（无LLM配置）
- ✅ 正常流程（单条/多条记录）
- ✅ 错误处理（无效日期、未知分类）
- ✅ 类型支持（收入/支出）
- ✅ 数据库操作（创建事务）

## 🚀 运行测试

### 运行所有测试
```bash
cd src-tauri
cargo test
```

### 运行特定测试
```bash
# 只运行快速记账相关测试
cargo test quick_booking

# 运行特定测试用例
cargo test test_process_quick_booking_text_success_single_transaction
```

### 运行时输出
- ✅ 14个测试全部通过
- ⚠️ 编译警告（未使用的导入/变量，不影响功能）

## 📊 测试结果

```
running 14 tests
test ai::agent::base::tests::test_agent_context ... ok
test ai::agent::base::tests::test_agent_result ... ok
test ai::agent::quick_note::tests::test_extract_json_from_response ... ok
test utils::http_client::tests::test_url_building ... ok
test ai::agent::base::tests::test_agent_factory ... ok
test utils::http_client::tests::test_client_creation ... ok
test ai::agent::quick_note::tests::test_validate_parse_result ... ok
test tests::quick_booking_tests::tests::test_process_quick_booking_text_no_llm_config ... ok
test tests::quick_booking_tests::tests::test_process_quick_booking_text_success_single_transaction ... ok
test tests::quick_booking_tests::tests::test_process_quick_booking_text_unknown_category ... ok
test tests::quick_booking_tests::tests::test_process_quick_booking_text_empty_input ... ok
test tests::quick_booking_tests::tests::test_process_quick_booking_text_invalid_date ... ok
test tests::quick_booking_tests::tests::test_process_quick_booking_text_multiple_transactions ... ok
test tests::quick_booking_tests::tests::test_process_quick_booking_text_income_transaction ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## 🔧 维护建议

1. **添加新功能时**: 为新的边界情况添加相应测试
2. **修改AI响应格式时**: 更新模拟响应JSON格式
3. **数据库模式变更时**: 更新测试数据库初始化逻辑
4. **性能优化**: 考虑添加性能基准测试

## 📋 待扩展测试场景

- [ ] 大量数据处理性能测试
- [ ] 并发请求处理测试  
- [ ] AI API超时/错误响应测试
- [ ] 数据库连接失败场景测试
- [ ] 内存使用情况测试

---

*最后更新: 2025-01-20* 