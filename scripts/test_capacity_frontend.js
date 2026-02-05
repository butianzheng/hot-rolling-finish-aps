// 测试产能池API调用
// 在浏览器控制台运行此脚本

console.log('🔍 开始测试产能池API...');
console.log('');

// 检查激活版本
const activeVersion = localStorage.getItem('activeVersionId');
console.log('📋 当前激活版本:', activeVersion || '未设置');
console.log('');

// 测试API调用
const testCapacityAPI = async () => {
  try {
    console.log('📡 调用 get_capacity_pools API...');
    console.log('参数:');
    console.log('  - machine_codes: ["H032", "H033", "H034"]');
    console.log('  - date_from: 2026-02-05');
    console.log('  - date_to: 2026-02-10');
    console.log('  - version_id:', activeVersion || '(使用默认激活版本)');
    console.log('');

    const params = {
      machine_codes: JSON.stringify(['H032', 'H033', 'H034']),
      date_from: '2026-02-05',
      date_to: '2026-02-10',
    };

    if (activeVersion) {
      params.version_id = activeVersion;
    }

    // 使用window.__TAURI__调用后端
    const result = await window.__TAURI__.tauri.invoke('get_capacity_pools', params);

    console.log('✅ API调用成功！');
    console.log('');
    console.log('📊 返回结果类型:', typeof result);

    if (typeof result === 'string') {
      console.log('⚠️  结果是字符串，尝试解析...');
      const parsed = JSON.parse(result);
      console.log('✅ 解析成功！');
      console.log('记录数:', parsed.length);
      console.log('');
      console.log('前3条记录:');
      console.table(parsed.slice(0, 3));
      return parsed;
    } else {
      console.log('记录数:', result.length);
      console.log('');
      console.log('前3条记录:');
      console.table(result.slice(0, 3));
      return result;
    }
  } catch (error) {
    console.error('❌ API调用失败:');
    console.error(error);
    console.log('');
    console.log('💡 可能的原因:');
    console.log('  1. 后端服务未启动');
    console.log('  2. 命令名称错误');
    console.log('  3. 参数格式不正确');
    console.log('  4. 权限问题');
  }
};

// 执行测试
testCapacityAPI();
