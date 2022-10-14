Authentication-Results: vla5-77e4a2c621ec.qloud-c.yandex.net; spf=pass (vla5-77e4a2c621ec.qloud-c.yandex.net: domain of yahoo.com designates 77.238.179.83 as permitted sender, rule=[ptr:yahoo.com]) smtp.mail=alice@yahoo.com; dkim=pass header.i=@yahoo.com
From: <alice@yahoo.com>
To: <alice@yandex.ru>
