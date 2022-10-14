Authentication-Results: vla5-30ef2e2d46cd.qloud-c.yandex.net; spf=pass (vla5-30ef2e2d46cd.qloud-c.yandex.net: domain of aol.com designates 77.238.176.206 as permitted sender, rule=[ip4:77.238.176.0/22]) smtp.mail=alice@aol.com; dkim=pass header.i=@aol.com
From: <alice@aol.com>
To: <alice@yandex.ru>
