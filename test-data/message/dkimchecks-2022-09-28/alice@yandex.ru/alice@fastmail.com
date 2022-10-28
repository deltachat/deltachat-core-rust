Authentication-Results: myt6-c6cdcba1eefd.qloud-c.yandex.net; spf=pass (myt6-c6cdcba1eefd.qloud-c.yandex.net: domain of fastmail.com designates 66.111.4.28 as permitted sender, rule=[ip4:66.111.4.28]) smtp.mail=alice@fastmail.com; dkim=pass header.i=@fastmail.com
From: <alice@fastmail.com>
To: <alice@yandex.ru>
