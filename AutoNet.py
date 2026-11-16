"""
TYUT 校园网自动登录脚本
加密: 单字节 XOR key=0x77 (secret_key='drcom' 的 XOR 聚合值)
流程: 访问 Portal → loadConfig → login

首次运行会提示输入账号密码，加密后保存到 config.enc，之后自动读取。
"""

import os, re, sys, socket, uuid, base64, time, json, struct
from datetime import datetime
from getpass import getpass

try:
    import httpx
except ImportError:
    print("[!] 请运行: pip install httpx")
    sys.exit(1)

CONFIG_FILE = os.path.join(os.path.dirname(__file__), "config.enc")

PORTAL_URL = "http://219.226.127.250"
LOGIN_HOST = "https://drcom.tyut.edu.cn:804"
TARGET = "223.5.5.5"
KEY = 0x77


def log(msg):
    print(f"[{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}] {msg}")


CAMPUS_DNS_SERVERS = ("202.207.240.225", "202.207.247.227")
CAMPUS_DNS_HOSTS = {"drcom.tyut.edu.cn"}
_DNS_CACHE = {}
_SYSTEM_GETADDRINFO = socket.getaddrinfo


def _skip_dns_name(packet, offset):
    while True:
        if offset >= len(packet):
            raise ValueError("truncated DNS name")
        length = packet[offset]
        if length == 0:
            return offset + 1
        if length & 0xC0 == 0xC0:
            if offset + 1 >= len(packet):
                raise ValueError("truncated DNS pointer")
            return offset + 2
        if length & 0xC0:
            raise ValueError("invalid DNS label")
        offset += length + 1


def _query_campus_dns(host, server):
    query_id = int.from_bytes(os.urandom(2), "big")
    labels = host.rstrip(".").encode("idna").split(b".")
    qname = b"".join(bytes((len(label),)) + label for label in labels) + b"\x00"
    packet = struct.pack("!HHHHHH", query_id, 0x0100, 1, 0, 0, 0)
    packet += qname + struct.pack("!HH", 1, 1)

    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as dns_socket:
        dns_socket.settimeout(2)
        dns_socket.connect((server, 53))
        dns_socket.send(packet)
        response = dns_socket.recv(4096)

    if len(response) < 12:
        raise ValueError("short DNS response")
    response_id, flags, question_count, answer_count, _, _ = struct.unpack_from("!HHHHHH", response)
    if response_id != query_id or not flags & 0x8000:
        raise ValueError("invalid DNS response")
    if flags & 0x000F:
        raise OSError("DNS server returned error code " + str(flags & 0x000F))

    offset = 12
    for _ in range(question_count):
        offset = _skip_dns_name(response, offset) + 4

    addresses = []
    ttls = []
    for _ in range(answer_count):
        offset = _skip_dns_name(response, offset)
        record_type, record_class, ttl, data_length = struct.unpack_from("!HHIH", response, offset)
        offset += 10
        data = response[offset:offset + data_length]
        offset += data_length
        if record_type == 1 and record_class == 1 and data_length == 4:
            addresses.append(socket.inet_ntoa(data))
            ttls.append(ttl)

    if not addresses:
        raise OSError("no IPv4 answer")
    return tuple(dict.fromkeys(addresses)), min(ttls, default=0)


def resolve_campus_host(host):
    normalized = host.rstrip(".").lower()
    cached = _DNS_CACHE.get(normalized)
    if cached and cached[0] > time.monotonic():
        return cached[1]

    errors = []
    for server in CAMPUS_DNS_SERVERS:
        try:
            addresses, ttl = _query_campus_dns(normalized, server)
            _DNS_CACHE[normalized] = (time.monotonic() + min(ttl, 300), addresses)
            log("校园 DNS " + server + " 解析 " + normalized + " -> " + ", ".join(addresses))
            return addresses
        except (OSError, ValueError, struct.error) as exc:
            errors.append(server + ": " + str(exc))

    raise socket.gaierror(socket.EAI_NONAME, "campus DNS lookup failed: " + "; ".join(errors))


def _getaddrinfo_with_campus_dns(host, port, family=0, type=0, proto=0, flags=0):
    normalized = host.decode("ascii", "ignore") if isinstance(host, bytes) else host
    if isinstance(normalized, str) and normalized.rstrip(".").lower() in CAMPUS_DNS_HOSTS:
        if family not in (0, socket.AF_UNSPEC, socket.AF_INET):
            raise socket.gaierror(socket.EAI_NONAME, "campus host has no requested address family")
        results = []
        for address in resolve_campus_host(normalized):
            results.extend(_SYSTEM_GETADDRINFO(address, port, socket.AF_INET, type, proto, flags))
        return results
    return _SYSTEM_GETADDRINFO(host, port, family, type, proto, flags)


socket.getaddrinfo = _getaddrinfo_with_campus_dns

def xor_encrypt(s):
    return "".join(f"{ord(c) ^ KEY:02x}" for c in s)


def get_local_ip():
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.settimeout(3)
    s.connect((TARGET, 80))
    ip = s.getsockname()[0]
    s.close()
    return ip


def get_mac_str():
    return '%012x' % uuid.getnode()


def check_network():
    """检查是否已认证：能直连外网 或 Portal 未重定向，都算已在线"""
    try:
        r = httpx.get(f"http://{TARGET}", timeout=3, follow_redirects=False, trust_env=False)
        if r.status_code == 200:
            return True
    except:
        pass
    # 如果访问 Portal 被重定向到成功页，也说明已在线
    try:
        r = httpx.get(PORTAL_URL, timeout=5, follow_redirects=False, trust_env=False)
        # 如果返回的不是登录页（没有 wlan_user_ip 等关键词），说明已在线
        if r.status_code in (301, 302):
            return True
        if 'v46ip' not in r.text and 'wlan_user_ip' not in r.text:
            return True
    except:
        pass
    return False


def load_credentials():
    """读取保存的加密凭证，返回 (encrypted_account, encrypted_password)"""
    if not os.path.exists(CONFIG_FILE):
        return None, None
    try:
        with open(CONFIG_FILE, "r") as f:
            data = json.load(f)
        acc = data.get("account")
        pwd = data.get("password")
        if acc and pwd:
            log(f"已读取保存的加密凭证 (账号加密值: {acc[:8]}...)")
            return acc, pwd
    except Exception as e:
        log(f"读取配置文件失败: {e}")
    return None, None


def save_credentials(account_enc, password_enc):
    """保存加密后的凭证到文件（仅存加密 hex，不存明文）"""
    data = {
        "account": account_enc,
        "password": password_enc,
        "saved_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
    }
    with open(CONFIG_FILE, "w") as f:
        json.dump(data, f, indent=2)
    log(f"加密凭证已保存到 {CONFIG_FILE}")


def prompt_credentials():
    """首次运行：交互式输入账号密码，加密后返回并保存"""
    print()
    print("=" * 50)
    print("首次运行，请输入校园网账号密码")
    print("（仅加密后保存到本地，不传输到第三方）")
    print("=" * 50)
    username = input("  账号: ").strip()
    password = getpass("  密码: ").strip()
    if not username or not password:
        print("账号密码不能为空")
        sys.exit(1)

    acc_enc = xor_encrypt(username)
    pwd_enc = xor_encrypt(password)
    save_credentials(acc_enc, pwd_enc)
    print()
    return acc_enc, pwd_enc


def get_portal_cookies_and_ip(client):
    """访问 Portal 获取 PHPSESSID 和真实 IP"""
    log(f"访问 Portal: {PORTAL_URL}")
    r = client.get(PORTAL_URL, timeout=10, follow_redirects=False)
    log(f"Portal 状态: {r.status_code}")

    if r.status_code in (301, 302, 307, 308) and 'location' in r.headers:
        loc = r.headers['location']
        log(f"跟随重定向到: {loc}")
        r = client.get(loc, timeout=10, follow_redirects=True)
        log(f"最终页面: {len(r.text)} 字节, cookies: {dict(client.cookies)}")
        html = r.text
    else:
        html = r.text

    os.makedirs("debug", exist_ok=True)
    with open("debug/portal_full.html", "w", encoding="utf-8") as f:
        f.write(html)

    user_ip = None
    for pat in [r'v46ip\s*[:=]\s*["\']([^"\']+)',
                r'wlan_user_ip["\']?\s*[:=]\s*["\']([^"\']+)']:
        m = re.search(pat, html)
        if m and m.group(1).count('.') == 3:
            user_ip = m.group(1)
            log(f"提取到 IP: {user_ip}")
            break

    return user_ip, dict(client.cookies)


def do_load_config(client, ip, mac):
    ip_b64 = base64.b64encode(ip.encode()).decode()
    ip_enc = xor_encrypt(ip_b64)
    mac_enc = xor_encrypt(mac)

    params = {
        "callback": xor_encrypt("dr1003"),
        "program_index": "",
        "wlan_vlan_id": xor_encrypt("46"),
        "wlan_user_ip": ip_enc,
        "wlan_user_ipv6": "",
        "wlan_user_ssid": "",
        "wlan_user_areaid": "",
        "wlan_ac_ip": "",
        "wlan_ap_mac": mac_enc,
        "gw_id": mac_enc,
        "jsVersion": xor_encrypt("4.X"),
        "encrypt": "1",
        "v": str(int(time.time()) % 10000 + 500),
        "lang": "zh",
    }
    headers = {
        "Accept": "*/*",
        "Referer": "https://drcom.tyut.edu.cn:804/eportal/portal/index",
        "User-Agent": "Mozilla/5.0 Chrome/146.0.0.0 Edg/146.0.0.0",
    }
    log("loadConfig ...")
    r = client.get(f"{LOGIN_HOST}/eportal/portal/page/loadConfig",
                   params=params, headers=headers, timeout=15)
    log(f"loadConfig 响应: {r.text[:300]}")
    return r


def do_login(client, ip, mac, acct_enc, pwd_enc):
    """发送登录请求（acct_enc 和 pwd_enc 已经是加密后的 hex）"""
    ua_plain = ("Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
                "AppleWebKit/537.36 (KHTML, like Gecko) "
                "Chrome/146.0.0.0 Safari/537.36 Edg/146.0.0.0")

    params = {
        "callback": xor_encrypt("dr1004"),
        "login_method": xor_encrypt("1"),
        "user_account": acct_enc,
        "user_password": pwd_enc,
        "wlan_user_ip": xor_encrypt(ip),
        "wlan_user_ipv6": "",
        "wlan_user_mac": xor_encrypt("000000000000"),
        "wlan_ac_ip": "",
        "wlan_ac_name": "",
        "mac_type": xor_encrypt("0"),
        "authex_enable": "",
        "jsVersion": xor_encrypt("4.3"),
        "web": xor_encrypt("0"),
        "terminal_type": xor_encrypt("1"),
        "lang": xor_encrypt("zh-cn"),
        "user_agent": xor_encrypt(ua_plain),
        "enable_r3": xor_encrypt("0"),
        "v": str(int(time.time()) % 10000 + 500),
        "encrypt": "1",
    }
    headers = {
        "Accept": "*/*",
        "Referer": "https://drcom.tyut.edu.cn:804/eportal/portal/index",
        "User-Agent": "Mozilla/5.0 Chrome/146.0.0.0 Edg/146.0.0.0",
    }
    log("login ...")
    r = client.get(f"{LOGIN_HOST}/eportal/portal/login",
                   params=params, headers=headers, timeout=15)
    text = r.text.strip()
    log(f"login 响应: {text[:300]}")

    is_success = ('认证成功' in text and '"result":1' in text) or \
                 ('已在线' in text) or \
                 ('认证超时' in text)  # 已在线时重复登录会超时
    if is_success:
        msg = "登录成功!" if "认证成功" in text else \
              "已在线" if "已在线" in text else \
              "已在线（认证超时，可能已有会话）"
        log("✓ " + msg)
        return True
    elif "密码错误" in text:
        log("✗ 密码错误")
        return False
    elif "异常" in text:
        log("? 异常: " + text[:100])
        return None
    else:
        log("? 未知: " + text[:100])
        return None


def reset_config():
    """删除配置文件，下次运行重新输入"""
    if os.path.exists(CONFIG_FILE):
        os.remove(CONFIG_FILE)
        log(f"已删除 {CONFIG_FILE}")
    else:
        log("配置文件不存在")


def attempt_login(acc_enc, pwd_enc):
    """执行一次完整的登录流程，成功返回 True"""
    local_ip = get_local_ip()
    local_mac = get_mac_str()
    log(f"本机 IP: {local_ip}  MAC: {local_mac}")

    with httpx.Client(trust_env=False) as client:
        user_ip, _ = get_portal_cookies_and_ip(client)
        if not user_ip:
            user_ip = local_ip
        log(f"最终使用 IP: {user_ip}")

        do_load_config(client, user_ip, local_mac)
        time.sleep(0.5)
        return do_login(client, user_ip, local_mac, acc_enc, pwd_enc)


def main():
    # --daemon: 后台模式，脱离控制台运行
    if "--daemon" in sys.argv:
        if os.name != "nt":
            print("Linux 请使用 tmux 启动：bash start-linux.sh")
            return
        import subprocess as _sp
        base = os.path.dirname(__file__)
        pid_file = os.path.join(base, "autonet.pid")
        # 尝试不同日志文件，避免被锁
        for tag in ["", f"_{os.getpid()}", ""]:
            log_file = os.path.join(base, f"autonet_daemon{tag}.log")
            if not tag:
                try:
                    if os.path.exists(log_file):
                        os.remove(log_file)
                except:
                    continue  # 被锁了，下一轮用带 pid 的文件名
            try:
                fh = open(log_file, "a")
                break
            except PermissionError:
                if tag:
                    raise
                continue
        args = [sys.executable, "-u", __file__] + [
            a for a in sys.argv if a != "--daemon"
        ]
        env = os.environ.copy()
        env["PYTHONUNBUFFERED"] = "1"
        p = _sp.Popen(
            args, stdout=fh, stderr=_sp.STDOUT,
            creationflags=_sp.DETACHED_PROCESS | _sp.CREATE_NO_WINDOW,
            env=env,
        )
        fh.close()
        with open(pid_file, "w") as f:
            f.write(str(p.pid))
        # pythonw 下无控制台 stdout，print 需容错
        try:
            print(f"Daemon started (PID: {p.pid}), log: {log_file}")
        except Exception:
            pass
        return

    # 处理 --once 参数（单次运行，不进入监控循环）
    run_once = "--once" in sys.argv

    if "--reset" in sys.argv:
        reset_config()
        return

    log("=" * 50)
    log("TYUT 校园网自动登录守护")
    log("=" * 50)

    # 读取或输入凭证
    acc_enc, pwd_enc = load_credentials()
    if not acc_enc or not pwd_enc:
        acc_enc, pwd_enc = prompt_credentials()

    retry_wait = 10  # 初始重试等待（秒）
    max_retry = 3600  # 最大等待

    while True:
        try:
            if check_network():
                retry_wait = 10  # 在线时重置等待时间
                if run_once:
                    log("网络已连通，无需登录")
                    return
                # 每 600 秒（10 分钟）检查一次
                log("网络正常，继续监控；下次检查约 10 分钟后")
                time.sleep(600)
                continue

            # 断网：先快速确认 2 次
            log("网络断开，正在确认...")
            time.sleep(1)
            if check_network():
                continue
            time.sleep(2)
            if check_network():
                continue

            # 确认断网，尝试登录
            log(f"尝试重连，等待 {retry_wait} 秒后重试...")
            time.sleep(retry_wait)
            result = attempt_login(acc_enc, pwd_enc)

            if result:
                log("✓ 网络已恢复")
                retry_wait = 10  # 重置
            else:
                retry_wait = min(retry_wait * 2, max_retry)  # 指数退避
                log(f"重连失败，下次等待 {retry_wait} 秒")

            if run_once:
                return
        except Exception as e:
            # 网络超时等异常不应终止守护进程，记录后退避重试
            log(f"! 守护异常（自动恢复）: {type(e).__name__}: {e}")
            retry_wait = min(retry_wait * 2, max_retry)
            time.sleep(30)
            continue


if __name__ == "__main__":
    main()
