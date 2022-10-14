Authentication-Results: vla5-bc29b3935b72.qloud-c.yandex.net; spf=pass (vla5-bc29b3935b72.qloud-c.yandex.net: domain of mailo.com designates 213.182.54.15 as permitted sender, rule=[ip4:213.182.54.0/24]) smtp.mail=alice@mailo.com; dkim=pass header.i=@mailo.com
From: <alice@mailo.com>
To: <alice@yandex.ru>
