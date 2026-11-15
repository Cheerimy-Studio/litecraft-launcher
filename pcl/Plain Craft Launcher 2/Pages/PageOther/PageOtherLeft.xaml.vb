Public Class PageOtherLeft

    Public PageID As FormMain.PageSubType = FormMain.PageSubType.OtherAbout

    Public Sub New()
        InitializeComponent()
    End Sub

    Private Sub PageOtherLeft_Loaded(sender As Object, e As RoutedEventArgs) Handles Me.Loaded
        ItemAbout.SetChecked(True, False, False)
    End Sub

    ''' <summary>
    ''' 勾选事件改变页面。
    ''' </summary>
    Private Sub PageCheck(sender As FrameworkElement, e As RouteEventArgs) Handles ItemAbout.Check
        If sender.Tag IsNot Nothing Then PageChange(FormMain.PageSubType.OtherAbout)
    End Sub

    Public Function PageGet(Optional ID As FormMain.PageSubType = -1)
        If FrmOtherAbout Is Nothing Then FrmOtherAbout = New PageOtherAbout
        Return FrmOtherAbout
    End Function

    ''' <summary>
    ''' 切换现有页面。
    ''' </summary>
    Public Sub PageChange(ID As FormMain.PageSubType)
        If PageID = ID AndAlso TypeOf FrmMain.PageRight Is PageOtherAbout Then Return
        AniControlEnabled += 1
        Try
            PageChangeRun(PageGet(ID))
            PageID = ID
        Catch ex As Exception
            Logger.Error(ex, $"切换分页面失败（ID {ID}）")
        Finally
            AniControlEnabled -= 1
        End Try
    End Sub

    Private Shared Sub PageChangeRun(Target As MyPageRight)
        AniStop("FrmMain PageChangeRight") '停止主页面的右页面切换动画，防止它与本动画一起触发多次 PageOnEnter
        If Target.Parent IsNot Nothing Then Target.SetValue(ContentPresenter.ContentProperty, Nothing)
        FrmMain.PageRight = Target
        CType(FrmMain.PanMainRight.Child, MyPageRight).PageOnExit()
        AniStart({
                         AaCode(Sub()
                                    CType(FrmMain.PanMainRight.Child, MyPageRight).PageOnForceExit()
                                    FrmMain.PanMainRight.Child = FrmMain.PageRight
                                    FrmMain.PageRight.Opacity = 0
                                End Sub, 130),
                         AaCode(Sub()
                                    '延迟触发页面通用动画，以使得在 Loaded 事件中加载的控件得以处理
                                    FrmMain.PageRight.Opacity = 1
                                    FrmMain.PageRight.PageOnEnter()
                                End Sub, 30, True)
                     }, "PageLeft PageChange")
    End Sub

    '强制刷新
    Public Sub Refresh(sender As Object, e As EventArgs) '由边栏按钮匿名调用
        Hint("正在刷新……", Log:=False)
    End Sub

    '帮助页面已移除；保留空实现，避免命令行 --help 等入口报错
    Public Shared Sub RefreshHelp()
    End Sub

    '反馈页面已移除；保留入口，统一转到项目 GitHub
    Public Shared Sub TryFeedback()
        OpenWebsite("https://github.com/Cheerimy-Studio/litecraft-launcher/issues")
    End Sub

End Class
