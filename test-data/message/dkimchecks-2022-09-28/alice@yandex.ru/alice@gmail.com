Authentication-Results: vla1-f55d97afef99.qloud-c.yandex.net; spf=pass (vla1-f55d97afef99.qloud-c.yandex.net: domain of gmail.com designates 2a00:1450:4864:20::443 as permitted sender, rule=[ip6:2a00:1450:4000::/36]) smtp.mail=alice@gmail.com; dkim=pass header.i=@gmail.com
From: <alice@gmail.com>
To: <alice@yandex.ru>
