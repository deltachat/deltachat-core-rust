Authentication-Results: sas1-fadc704f0d28.qloud-c.yandex.net; spf=pass (sas1-fadc704f0d28.qloud-c.yandex.net: domain of icloud.com designates 17.57.155.16 as permitted sender, rule=[ip4:17.57.155.0/24]) smtp.mail=alice@icloud.com; dkim=pass header.i=@icloud.com
From: <alice@icloud.com>
To: <alice@yandex.ru>
