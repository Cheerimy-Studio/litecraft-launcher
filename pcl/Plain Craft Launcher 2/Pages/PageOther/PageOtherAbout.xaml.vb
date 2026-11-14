Public Class PageOtherAbout

    Private Shadows IsLoaded As Boolean = False
    Private Sub PageOtherAbout_Loaded(sender As Object, e As RoutedEventArgs) Handles Me.Loaded

        '重复加载部分
        PanBack.ScrollToHome()

        '非重复加载部分
        If IsLoaded Then Return
        IsLoaded = True

        ItemAboutPcl.Info = ItemAboutPcl.Info.Replace("%VERSION%", VersionDisplay).Replace("%VERSIONCODE%", VersionCode).Replace("%BRANCH%", CInt(BuildType))

    End Sub

    Public Shared Sub CopyIdentify()
        ClipboardSet(Identify)
    End Sub

    Private Sub BtnAboutUpdate_Click(sender As Object, e As MouseButtonEventArgs) Handles BtnAboutUpdate.Click
        UpdateCheckByButton()
    End Sub

    Private Sub BtnAboutSponsor_Click(sender As Object, e As MouseButtonEventArgs) Handles BtnAboutSponsor.Click
        MyMsgBox("本项目为 LiteCraft Launcher，基于 PCL2 开发，保留原作者的赞助入口。" & vbCrLf &
                 "赞助 PCL2 不会为 LiteCraft 的开发提供任何帮助，请慎重考虑。" & vbCrLf &
                 "LiteCraft 不对你的赞助行为承担任何责任认定。",
                 "关于赞助 LiteCraft 依赖项目的相关通知", "我知道了", IsWarn:=True)
    End Sub

End Class
