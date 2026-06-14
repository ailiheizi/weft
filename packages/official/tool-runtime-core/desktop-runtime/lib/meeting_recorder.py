"""
长征机会议录音器 v4.0
录音策略（按优先级）：
  1. pyaudiowpatch WASAPI Loopback — 系统输出环回，不干扰任何应用，
     自动跟随当前默认扬声器（蓝牙/Realtek 均可）
  2. sounddevice + 立体声混音 (Stereo Mix) 共享模式 — Realtek 系统音频
  3. sounddevice + 默认输入麦克风

依赖：
  pip install pyaudiowpatch   (已安装)
  pip install sounddevice numpy  (已内置)
用法：python meeting_recorder.py --output <path.wav>
"""
import sys
import os
import wave
import signal
import argparse
import threading
import time

try:
    import numpy as np
except ImportError:
    print('[recorder] ERROR: pip install numpy', flush=True)
    sys.exit(1)

SAMPLE_RATE = 16000
CHANNELS    = 1
CHUNK_SIZE  = 1024

_running = True
_lock    = threading.Lock()
_buf     = []


def _stop(*_):
    global _running
    _running = False


signal.signal(signal.SIGTERM, _stop)
signal.signal(signal.SIGINT,  _stop)


def _stdin_watcher():
    try:
        for line in sys.stdin:
            if line.strip().lower() == 'stop':
                _stop()
                break
    except Exception:
        pass


# ─────────────────────────────────────────────
# 策略 1：pyaudiowpatch WASAPI Loopback
# ─────────────────────────────────────────────
def _record_pyaudiowpatch(output_path):
    """
    使用 pyaudiowpatch 进行 WASAPI 环回录音。
    成功返回 True，不可用时返回 False（让调用方 fallback）。
    """
    try:
        import pyaudiowpatch as pyaudio
    except ImportError:
        print('[recorder] pyaudiowpatch 未安装，跳过', flush=True)
        return False

    pa = pyaudio.PyAudio()
    loopback_device = None

    try:
        # 找 WASAPI 主机 API
        wasapi_idx = None
        for i in range(pa.get_host_api_count()):
            info = pa.get_host_api_info_by_index(i)
            if info.get('type') == pyaudio.paWASAPI:
                wasapi_idx = i
                break

        if wasapi_idx is None:
            print('[recorder] 未找到 WASAPI 主机 API，跳过 pyaudiowpatch', flush=True)
            pa.terminate()
            return False

        wasapi_info = pa.get_host_api_info_by_index(wasapi_idx)
        default_out_idx = wasapi_info.get('defaultOutputDevice', -1)
        if default_out_idx < 0:
            print('[recorder] 未找到默认输出设备，跳过', flush=True)
            pa.terminate()
            return False

        default_out = pa.get_device_info_by_index(default_out_idx)
        default_out_name = default_out.get('name', '')
        print(f'[recorder] 默认输出设备: {default_out_name} (idx={default_out_idx})', flush=True)

        # 枚举 loopback 设备，选与默认输出名称最接近的
        for i in range(pa.get_device_count()):
            dev = pa.get_device_info_by_index(i)
            if dev.get('isLoopbackDevice', False):
                if default_out_name and default_out_name[:10] in dev.get('name', ''):
                    loopback_device = dev
                    loopback_device['_index'] = i
                    break

        # 如果没找到匹配，就取第一个 loopback
        if loopback_device is None:
            for i in range(pa.get_device_count()):
                dev = pa.get_device_info_by_index(i)
                if dev.get('isLoopbackDevice', False):
                    loopback_device = dev
                    loopback_device['_index'] = i
                    break

        if loopback_device is None:
            print('[recorder] 未找到任何 Loopback 设备，跳过 pyaudiowpatch', flush=True)
            pa.terminate()
            return False

        lb_idx  = loopback_device['_index']
        lb_name = loopback_device.get('name', '')
        lb_ch   = int(loopback_device.get('maxInputChannels', 2))
        lb_rate = int(loopback_device.get('defaultSampleRate', SAMPLE_RATE))
        lb_ch   = max(1, min(lb_ch, 2))

        print(f'[recorder] 使用 Loopback: {lb_name} (idx={lb_idx})', flush=True)

    except Exception as e:
        print(f'[recorder] pyaudiowpatch 枚举失败: {e}，跳过', flush=True)
        try: pa.terminate()
        except: pass
        return False

    # 打开流并录音
    pcm_frames = []
    lock2 = threading.Lock()

    def pa_callback(in_data, frame_count, time_info, status):
        if _running:
            with lock2:
                pcm_frames.append(in_data)
        return (None, pyaudio.paContinue)

    try:
        stream = pa.open(
            format=pyaudio.paInt16,
            channels=lb_ch,
            rate=lb_rate,
            input=True,
            input_device_index=lb_idx,
            frames_per_buffer=CHUNK_SIZE,
            stream_callback=pa_callback,
        )
    except Exception as e:
        print(f'[recorder] pyaudiowpatch 打开流失败: {e}', flush=True)
        try: pa.terminate()
        except: pass
        return False

    stream.start_stream()
    print(f'[recorder] START device={lb_name} (pyaudiowpatch, rate={lb_rate}Hz)', flush=True)

    try:
        while _running:
            time.sleep(0.1)
    finally:
        stream.stop_stream()
        stream.close()
        pa.terminate()

    with lock2:
        chunks = list(pcm_frames)

    if not chunks:
        print('[recorder] ERROR 录音数据为空', flush=True)
        sys.exit(1)

    # 拼接 + 转为 numpy（int16）
    raw = b''.join(chunks)
    pcm = np.frombuffer(raw, dtype=np.int16)
    if lb_ch > 1:
        pcm = pcm.reshape(-1, lb_ch)[:, 0]  # 取第一声道

    # 如果设备采样率不是 16kHz，降采样
    if lb_rate != SAMPLE_RATE:
        from fractions import Fraction
        ratio = Fraction(SAMPLE_RATE, lb_rate).limit_denominator(100)
        # 简单线性插值降采样
        orig_len = len(pcm)
        new_len  = int(orig_len * ratio.numerator / ratio.denominator)
        pcm = np.interp(
            np.linspace(0, orig_len - 1, new_len),
            np.arange(orig_len),
            pcm.astype(np.float32),
        ).astype(np.int16)

    _save_wav(output_path, pcm)
    return True


# ─────────────────────────────────────────────
# 策略 2 & 3：sounddevice fallback
# ─────────────────────────────────────────────
def _record_sounddevice(output_path):
    try:
        import sounddevice as sd
    except ImportError:
        print('[recorder] ERROR: pip install sounddevice', flush=True)
        sys.exit(1)

    def callback(indata, frames, time_info, status):
        if _running:
            mono = indata[:, 0:1] if indata.shape[1] > 1 else indata
            with _lock:
                _buf.append(mono.copy())

    def try_stream(device, ch, extra, label):
        for c in ([min(ch, 2), 1] if ch > 1 else [1]):
            try:
                kw = dict(samplerate=SAMPLE_RATE, channels=c, dtype='int16',
                          device=device, latency='high', callback=callback)
                if extra is not None:
                    kw['extra_settings'] = extra
                s = sd.InputStream(**kw)
                s.start()
                return s, f'{label} (ch={c})'
            except Exception as e:
                print(f'[recorder] 尝试失败 {label}: {e}', flush=True)
        return None, ''

    candidates = []

    # 立体声混音（共享模式）
    try:
        ws = sd.WasapiSettings(exclusive=False)
        kws = ['stereo mix', 'loopback', '立体声混音']
        devs = sd.query_devices()
        for i, d in enumerate(devs):
            if d['max_input_channels'] < 1:
                continue
            try:
                nl = d['name'].lower()
            except Exception:
                nl = ''
            if any(k in nl for k in kws):
                candidates.append((i, d['max_input_channels'], ws, f'StereoMix:{d["name"]}'))
    except Exception:
        pass

    # 默认输入（共享）
    try:
        ws = sd.WasapiSettings(exclusive=False)
        candidates.append((None, 2, ws, '默认输入(WASAPI共享)'))
    except Exception:
        pass

    # 最终 fallback（MME，始终共享）
    candidates.append((None, 2, None, '默认输入(MME)'))

    stream = None
    label  = ''
    for dev, ch, extra, lbl in candidates:
        print(f'[recorder] 尝试: {lbl}', flush=True)
        stream, label = try_stream(dev, ch, extra, lbl)
        if stream is not None:
            break

    if stream is None:
        print('[recorder] ERROR 所有录音方案均失败，请检查音频设备', flush=True)
        sys.exit(1)

    print(f'[recorder] START device={label}', flush=True)
    try:
        while _running:
            time.sleep(0.1)
    finally:
        try: stream.stop(); stream.close()
        except: pass

    with _lock:
        chunks = list(_buf)

    if not chunks:
        print('[recorder] ERROR 录音数据为空', flush=True)
        sys.exit(1)

    pcm = np.concatenate(chunks, axis=0)[:, 0]
    _save_wav(output_path, pcm)


# ─────────────────────────────────────────────
# 工具
# ─────────────────────────────────────────────
def _save_wav(output_path, pcm):
    with wave.open(output_path, 'wb') as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(SAMPLE_RATE)
        wf.writeframes(pcm.tobytes())
    duration = len(pcm) / SAMPLE_RATE
    print(f'[recorder] DONE {duration:.1f}s', flush=True)


def record(output_path):
    t_stdin = threading.Thread(target=_stdin_watcher, daemon=True)
    t_stdin.start()

    # 策略 1：pyaudiowpatch WASAPI Loopback
    if _record_pyaudiowpatch(output_path):
        return

    # 策略 2 & 3：sounddevice fallback
    global _buf
    _buf = []
    _record_sounddevice(output_path)


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('--output', required=True, help='输出 WAV 文件路径')
    args = ap.parse_args()
    record(args.output)
