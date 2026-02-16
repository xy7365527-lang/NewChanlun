#!/usr/bin/env python3
"""
DataBento 数据获取脚本

用途：从 DataBento API 获取美股分钟级 OHLCV 数据，用于缠论引擎验证。

环境要求：
- 需要在项目根目录的 .env 文件中配置 DATABENTO_API_KEY
- 需要安装 databento 包: pip install databento
- 需要安装 python-dotenv 包: pip install python-dotenv

使用方法：
    python scripts/fetch_databento.py [symbol] [days_back]
    
参数：
    symbol: 股票代码，默认为 AAPL
    days_back: 获取最近几个交易日的数据，默认为 5
    
示例：
    python scripts/fetch_databento.py AAPL 5
    python scripts/fetch_databento.py TSLA 10
"""

import os
import sys
from pathlib import Path
from datetime import datetime, timedelta
import pandas as pd

# 确保项目根目录在 Python 路径中
project_root = Path(__file__).parent.parent
sys.path.insert(0, str(project_root))

# 加载环境变量
try:
    from dotenv import load_dotenv
    load_dotenv(project_root / '.env')
except ImportError:
    print("警告: python-dotenv 未安装，尝试直接从环境变量读取 API key")
    print("建议安装: pip install python-dotenv")

# 导入 databento
try:
    import databento as db
except ImportError:
    print("错误: databento 包未安装")
    print("请运行: pip install databento")
    sys.exit(1)


def fetch_market_data(symbol: str = "AAPL", days_back: int = 5, max_retries: int = 2) -> pd.DataFrame:
    """
    从 DataBento 获取指定股票的分钟级 OHLCV 数据
    
    Args:
        symbol: 股票代码
        days_back: 往回获取的天数（自然日）
        max_retries: 最大重试次数
        
    Returns:
        包含 timestamp, open, high, low, close, volume 列的 DataFrame
    """
    # 获取 API key
    api_key = os.environ.get('DATABENTO_API_KEY')
    if not api_key:
        raise ValueError(
            "未找到 DATABENTO_API_KEY 环境变量。\n"
            "请在项目根目录的 .env 文件中配置:\n"
            "DATABENTO_API_KEY=your_api_key_here"
        )
    
    # 计算日期范围（历史数据通常有1-2天延迟，所以 end_date 使用 today - 2 天）
    end_date = (datetime.now() - timedelta(days=2)).date()
    start_date = end_date - timedelta(days=days_back)
    
    print(f"正在获取 {symbol} 数据...")
    print(f"日期范围: {start_date} 至 {end_date}")
    print(f"API Key 前缀: {api_key[:10]}...")
    
    retry_count = 0
    last_error = None
    
    while retry_count <= max_retries:
        try:
            # 创建 DataBento 客户端
            client = db.Historical(api_key)
            
            # 获取 OHLCV-1m 数据（1分钟聚合）
            # dataset: 使用 XNAS.ITCH (Nasdaq) 或 GLBX.MDP3 (CME)
            # 对于美股，使用 XNAS.ITCH
            data = client.timeseries.get_range(
                dataset='XNAS.ITCH',
                symbols=[symbol],
                schema='ohlcv-1m',
                start=start_date.isoformat(),
                end=end_date.isoformat(),
                stype_in='raw_symbol',
            )
            
            # 转换为 DataFrame
            df = data.to_df()
            
            if df.empty:
                print(f"警告: 未获取到任何数据。可能原因：")
                print(f"  1. 日期范围内无交易数据")
                print(f"  2. 股票代码 {symbol} 不正确")
                print(f"  3. 该股票在 XNAS.ITCH 数据集中不可用")
                return pd.DataFrame()
            
            # 重置索引，将 timestamp 变为列
            df = df.reset_index()
            
            # 选择需要的列并重命名
            # DataBento 的列名可能是: ts_event, open, high, low, close, volume
            df = df.rename(columns={
                'ts_event': 'timestamp'
            })
            
            # 确保包含必需的列
            required_columns = ['timestamp', 'open', 'high', 'low', 'close', 'volume']
            df = df[required_columns]
            
            # 转换时间戳格式（从纳秒转为可读格式）
            if df['timestamp'].dtype == 'int64':
                df['timestamp'] = pd.to_datetime(df['timestamp'], unit='ns')
            
            print(f"成功获取 {len(df)} 条数据记录")
            print(f"数据范围: {df['timestamp'].min()} 至 {df['timestamp'].max()}")
            
            return df
            
        except Exception as e:
            last_error = e
            retry_count += 1
            
            if retry_count <= max_retries:
                print(f"获取数据失败 (尝试 {retry_count}/{max_retries + 1}): {e}")
                print(f"等待 2 秒后重试...")
                import time
                time.sleep(2)
            else:
                print(f"\n错误: 达到最大重试次数 ({max_retries})")
                print(f"最后错误信息: {e}")
                print("\n可能的解决方案:")
                print("1. 检查 API key 是否正确")
                print("2. 检查网络连接")
                print("3. 检查股票代码是否正确")
                print("4. 检查 DataBento 账户权限和配额")
                raise last_error


def save_to_csv(df: pd.DataFrame, symbol: str, output_dir: Path) -> Path:
    """
    将数据保存为 CSV 文件
    
    Args:
        df: 数据 DataFrame
        symbol: 股票代码
        output_dir: 输出目录
        
    Returns:
        保存的文件路径
    """
    # 创建输出目录
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # 生成文件名（包含日期范围）
    start_date = df['timestamp'].min().strftime('%Y%m%d')
    end_date = df['timestamp'].max().strftime('%Y%m%d')
    filename = f"{symbol}_{start_date}_{end_date}_1m.csv"
    filepath = output_dir / filename
    
    # 保存为 CSV
    df.to_csv(filepath, index=False)
    print(f"数据已保存至: {filepath}")
    print(f"文件大小: {filepath.stat().st_size / 1024:.2f} KB")
    
    return filepath


def main():
    """主函数"""
    # 解析命令行参数
    symbol = sys.argv[1] if len(sys.argv) > 1 else "AAPL"
    days_back = int(sys.argv[2]) if len(sys.argv) > 2 else 5
    
    print("=" * 60)
    print("DataBento 数据获取脚本")
    print("=" * 60)
    
    try:
        # 获取数据
        df = fetch_market_data(symbol=symbol, days_back=days_back)
        
        if df.empty:
            print("\n未获取到数据，程序退出")
            return 1
        
        # 保存数据
        output_dir = project_root / 'data'
        filepath = save_to_csv(df, symbol, output_dir)
        
        # 显示数据预览
        print("\n数据预览（前5行）:")
        print(df.head())
        
        print("\n数据统计:")
        print(df[['open', 'high', 'low', 'close', 'volume']].describe())
        
        print("\n" + "=" * 60)
        print("数据获取完成！")
        print("=" * 60)
        
        return 0
        
    except Exception as e:
        print(f"\n程序执行失败: {e}")
        import traceback
        traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(main())
